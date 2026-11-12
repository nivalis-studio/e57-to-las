use std::ops::ControlFlow;
use std::path::Path;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

use crate::get_las_writer::{PointBounds, get_las_writer};
use crate::staged_las::{LasWriter, StagedLas};
use crate::{LasVersion, convert_point::convert_point, utils::ensure_parent_dir};

use anyhow::{Context, Result};
use e57::{E57Reader, PointCloud};
use rayon::prelude::*;

/// What a LAS writer needs to know about a single E57 point cloud before the
/// first point is written.
struct CloudStats {
    bounds: PointBounds,
    has_color: bool,
    skipped_points: usize,
}

/// Opens its own `E57Reader` on `input_path` and calls `on_point` for every
/// point [`convert_point`] accepts.
///
/// Returns how many points were skipped because of invalid coordinates, and
/// whether any source point carried color. Skipped points count for color too:
/// [`scan_pointcloud`] uses that flag to choose the LAS point format, and a
/// cloud can hold a colored point whose coordinates are invalid.
///
/// `on_point` returns [`ControlFlow::Break`] to stop before the next point. The
/// skipped count then covers only the points read so far.
fn for_each_las_point(
    input_path: &Path,
    pointcloud: &PointCloud,
    mut on_point: impl FnMut(las::Point) -> Result<ControlFlow<()>>,
) -> Result<(usize, bool)> {
    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    let mut skipped_points = 0;
    let mut has_color = false;
    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;
        if point.color.is_some() {
            has_color = true;
        }

        let Some(las_point) = convert_point(point) else {
            skipped_points += 1;
            continue;
        };
        if on_point(las_point)?.is_break() {
            break;
        }
    }

    Ok((skipped_points, has_color))
}

/// Pass 1: scans a single point cloud without keeping its points.
///
/// Opens its own `E57Reader` on `input_path` so that callers can safely invoke it
/// from parallel workers. Tracks the per-axis bounds of the converted points
/// (used to derive the LAS offset and scale), whether any point carries color,
/// and how many points were skipped because of invalid coordinates.
fn scan_pointcloud(input_path: &Path, pointcloud: &PointCloud) -> Result<CloudStats> {
    let mut bounds = PointBounds::default();
    let (skipped_points, has_color) = for_each_las_point(input_path, pointcloud, |las_point| {
        bounds.update(&las_point);
        Ok(ControlFlow::Continue(()))
    })?;

    Ok(CloudStats {
        bounds,
        has_color,
        skipped_points,
    })
}

/// Pass 2: re-reads a single point cloud and streams its points to `writer`.
///
/// Points rejected by `convert_point` are skipped; [`scan_pointcloud`] applies
/// the same rule, so they were already counted there.
fn write_pointcloud(
    input_path: &Path,
    pointcloud: &PointCloud,
    writer: &mut LasWriter,
    has_color: bool,
) -> Result<()> {
    for_each_las_point(input_path, pointcloud, |mut las_point| {
        backfill_color(&mut las_point, has_color);
        writer.write_point(las_point).context("Unable to write: ")?;
        Ok(ControlFlow::Continue(()))
    })?;

    Ok(())
}

/// Opens `staged`'s writer on the first point, with bounds holding only that
/// point, then writes `las_point` if it fits that writer's transform.
///
/// Returns `Ok(false)` when the point lies too far from the offset to fit in an
/// i32 at the writer's scale (more than ~2,147 km). The caller then discards
/// `staged` and retries with two passes.
fn write_if_point_fits(
    staged: &mut StagedLas,
    mut las_point: las::Point,
    has_color: bool,
    create_writer: &mut impl FnMut(PointBounds) -> Result<LasWriter>,
) -> Result<bool> {
    if !staged.has_writer() {
        let mut bounds = PointBounds::default();
        bounds.update(&las_point);
        staged.insert_writer(create_writer(bounds)?)?;
    }
    let writer = staged.writer_mut()?;

    let t = writer.header().transforms();
    let fits = t.x.inverse(las_point.x).is_ok()
        && t.y.inverse(las_point.y).is_ok()
        && t.z.inverse(las_point.z).is_ok();
    if !fits {
        return Ok(false);
    }

    backfill_color(&mut las_point, has_color);
    writer.write_point(las_point).context("Unable to write: ")?;
    Ok(true)
}

/// One pass: decodes `pointcloud` once and streams its valid points to `staged`.
///
/// The writer is created at the first valid point, with bounds holding only
/// that point: the LAS offset is that point rounded to meters and the scale is
/// 0.001. Every point is then checked against the writer's own transform.
///
/// Returns the number of points skipped because of invalid coordinates, or
/// `None` if a point lies too far from the offset to fit in i32 at that scale
/// (more than ~2,147 km). The caller closes and discards `staged`, then redoes
/// the conversion with the two-pass functions.
fn stream_pointcloud(
    input_path: &Path,
    pointcloud: &PointCloud,
    staged: &mut StagedLas,
    has_color: bool,
    create_writer: &mut impl FnMut(PointBounds) -> Result<LasWriter>,
) -> Result<Option<usize>> {
    let mut fits = true;
    let (skipped_points, _) = for_each_las_point(input_path, pointcloud, |las_point| {
        if !write_if_point_fits(staged, las_point, has_color, create_writer)? {
            fits = false;
            return Ok(ControlFlow::Break(()));
        }
        Ok(ControlFlow::Continue(()))
    })?;

    if !fits {
        return Ok(None);
    }
    Ok(Some(skipped_points))
}

/// Points per batch sent from a decoder to the merged writer.
pub(crate) const BATCH_SIZE: usize = 4096;
/// Batches the merged channel holds before decoders block.
///
/// Worst-case points in memory: `(CHANNEL_CAPACITY + threads + 1) × BATCH_SIZE ×
/// size_of::<las::Point>()`: the queued batches, one being filled per decoder
/// thread and one being written.
const CHANNEL_CAPACITY: usize = 16;

/// Decoder side of [`convert_pointclouds`]: decodes `pointcloud` once and sends
/// its valid points to the writer in batches of [`BATCH_SIZE`].
///
/// Stops quietly if the writer hung up (overflow or write error); the writer
/// reports why. Prints the skipped-points count once the cloud is fully decoded.
fn send_pointcloud(
    index: usize,
    input_path: &Path,
    pointcloud: &PointCloud,
    sender: &SyncSender<Vec<las::Point>>,
) -> Result<()> {
    let mut stopped = false;
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let (skipped_points, _) = for_each_las_point(input_path, pointcloud, |las_point| {
        batch.push(las_point);
        if batch.len() == BATCH_SIZE {
            let full = std::mem::replace(&mut batch, Vec::with_capacity(BATCH_SIZE));
            if sender.send(full).is_err() {
                stopped = true;
                return Ok(ControlFlow::Break(()));
            }
        }
        Ok(ControlFlow::Continue(()))
    })?;
    if stopped {
        return Ok(());
    }
    if !batch.is_empty() && sender.send(batch).is_err() {
        return Ok(());
    }

    if skipped_points > 0 {
        println!("Pointcloud {index}: skipped {skipped_points} points with invalid coordinates");
    }
    Ok(())
}

/// Writer side of [`convert_pointclouds`]: writes every received point until all
/// senders are dropped.
///
/// The writer is created at the first point received, with bounds holding only
/// that point (offset = that point rounded to meters, scale 0.001), and every
/// point is checked against its transform. Returns `Ok(false)` if a point does
/// not fit (more than ~2,147 km away). Dropping `receiver` stops the decoders.
/// The caller then closes `staged` and either publishes it or deletes it.
/// With no point at all, opens an empty file and leaves it for the caller.
fn write_batches(
    receiver: Receiver<Vec<las::Point>>,
    has_color: bool,
    staged: &mut StagedLas,
    mut create_writer: impl FnMut(PointBounds) -> Result<LasWriter>,
) -> Result<bool> {
    for batch in receiver {
        for las_point in batch {
            if !write_if_point_fits(staged, las_point, has_color, &mut create_writer)? {
                return Ok(false);
            }
        }
    }

    if !staged.has_writer() {
        staged.insert_writer(create_writer(PointBounds::default())?)?;
    }
    Ok(true)
}

/// Closes `staged` and deletes its temporary file so a later attempt can reuse
/// the same name.
fn discard_partial(mut staged: StagedLas) -> Result<()> {
    staged.close_writer()?;
    drop(staged);
    Ok(())
}

fn prepare_staged(output_path: &Path, file_name: &str) -> Result<StagedLas> {
    let final_path = ensure_parent_dir(output_path.join("las").join(file_name))
        .context("Unable to create path: ")?;
    Ok(StagedLas::new(final_path))
}

const FALLBACK_MESSAGE: &str =
    "Coordinates span more than 2147 km from the first point; retrying with a coarser scale";

/// Fails if `las_version` cannot store point colors but any cloud declares color,
/// so the conversion stops before reading any point.
pub(crate) fn check_color_support(
    pointclouds: &[PointCloud],
    las_version: &LasVersion,
) -> Result<()> {
    if !las_version.supports_color() && pointclouds.iter().any(PointCloud::has_color) {
        return Err(anyhow::anyhow!(
            "LAS {las_version} cannot store point colors, but the input contains colored point clouds. Use LAS version 1.2 or newer."
        ));
    }
    Ok(())
}

/// Converts a point cloud to a LAS file.
///
/// This function takes the points from the point cloud, converts them to LAS points using the
/// `convert_point` function, and streams them to `<output_path>/las/<index>.las`.
///
/// The cloud is decoded once: the LAS offset is the first valid point (rounded to
/// meters), the scale is 0.001 and the point format has color if the E57 prototype
/// declares color. If a point lies more than ~2,147 km from that offset, the file
/// is rewritten in two passes (bounds first, then points) with a coarser scale.
///
/// The LAS file appears in the output directory only after every point has been
/// written. A failure removes the partial file.
///
/// # Parameters
/// - `index`: The index of the point cloud.
/// - `pointcloud`: A reference to the point cloud to be converted.
/// - `input_path`: A reference to the input file path (E57 file).
/// - `output_path`: A reference to the output dir.
/// - `las_version`: The LAS version used for the output file.
///
/// # Example
/// ```ignore
/// use std::path::Path;
/// use e57_to_las::{convert_pointcloud, LasVersion};
///
/// # fn example() -> anyhow::Result<()> {
/// let input_path = Path::new("path/to/input.e57");
/// let output_path = Path::new("path/to/output");
/// let las_version = LasVersion::new(1, 4)?;
/// // pointcloud would be obtained from E57Reader in practice
/// # let pointcloud = todo!();
/// convert_pointcloud(0, &pointcloud, input_path, output_path, &las_version)?;
/// # Ok(())
/// # }
/// ```
pub fn convert_pointcloud(
    index: usize,
    pointcloud: &PointCloud,
    input_path: &Path,
    output_path: &Path,
    las_version: &LasVersion,
) -> Result<()> {
    check_color_support(std::slice::from_ref(pointcloud), las_version)?;

    let has_color = pointcloud.has_color();
    let mut staged = prepare_staged(output_path, &format!("{index}.las"))?;
    let temp_path = staged.temp_path().to_path_buf();
    let mut create_writer = |bounds| {
        get_las_writer(
            pointcloud.guid.clone(),
            temp_path.clone(),
            bounds,
            has_color,
            las_version,
        )
        .context("Unable to create writer: ")
    };

    let Some(skipped_points) = stream_pointcloud(
        input_path,
        pointcloud,
        &mut staged,
        has_color,
        &mut create_writer,
    )?
    else {
        // Drop the partial file before the fallback writes a new one.
        discard_partial(staged)?;
        println!("Pointcloud {index}: {FALLBACK_MESSAGE}");
        return convert_pointcloud_two_pass(
            index,
            pointcloud,
            input_path,
            output_path,
            las_version,
        );
    };

    if skipped_points > 0 {
        println!("Pointcloud {index}: skipped {skipped_points} points with invalid coordinates");
    }

    // No valid point: still write an (empty) LAS file.
    if !staged.has_writer() {
        staged.insert_writer(create_writer(PointBounds::default())?)?;
    }
    staged.commit()
}

/// Two-pass fallback of [`convert_pointcloud`]: scans the cloud for its bounds,
/// then re-reads it to write the points with a transform that fits them.
fn convert_pointcloud_two_pass(
    index: usize,
    pointcloud: &PointCloud,
    input_path: &Path,
    output_path: &Path,
    las_version: &LasVersion,
) -> Result<()> {
    let cloud = scan_pointcloud(input_path, pointcloud)?;

    if cloud.skipped_points > 0 {
        println!(
            "Pointcloud {index}: skipped {} points with invalid coordinates",
            cloud.skipped_points
        );
    }

    let mut staged = prepare_staged(output_path, &format!("{index}.las"))?;
    let temp_path = staged.temp_path().to_path_buf();
    staged.insert_writer(
        get_las_writer(
            pointcloud.guid.clone(),
            temp_path,
            cloud.bounds,
            cloud.has_color,
            las_version,
        )
        .context("Unable to create writer: ")?,
    )?;

    write_pointcloud(
        input_path,
        pointcloud,
        staged.writer_mut()?,
        cloud.has_color,
    )?;

    staged.commit()
}

/// Backfills a default (black) color on points missing one when the LAS point
/// format includes color, since `las` rejects points whose color presence does
/// not match the point format.
fn backfill_color(point: &mut las::Point, has_color: bool) {
    if has_color && point.color.is_none() {
        point.color = Some(las::Color::default());
    }
}

/// Converts all point clouds of an E57 file to a single merged LAS file.
///
/// This function decodes the point clouds of the E57 file at `input_path` once,
/// in parallel on the current rayon pool, converts the points to LAS points using
/// the `convert_point` function, and streams them through a bounded queue to a
/// single writer thread for `<output_path>/las/0.las`. Only a few batches of
/// points are held in memory. Points of different clouds may be interleaved and
/// their order may differ between runs. The LAS offset is the first valid point
/// the writer receives (rounded to meters), the scale is 0.001 and the point
/// format has color if any E57 prototype declares color. If a point lies more
/// than ~2,147 km from that offset, the whole file is rewritten in two passes
/// with a coarser scale (ordered by cloud). The LAS file appears in the output
/// directory only after every cloud has been written; a failure removes the
/// partial file.
///
/// This function is internal to the crate; use [`crate::convert_file`] with
/// `as_stations = false` for the public merged-conversion entry point.
///
/// # Parameters
/// - `input_path`: A reference to the input file path (E57 file).
/// - `output_path`: A reference to the output dir.
/// - `las_version`: The LAS version used for the output file.
pub(crate) fn convert_pointclouds(
    input_path: &Path,
    output_path: &Path,
    las_version: &LasVersion,
) -> Result<()> {
    let e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;
    let pointclouds = e57_reader.pointclouds();
    let guid = e57_reader.guid().to_owned();
    drop(e57_reader);

    let has_color = pointclouds.iter().any(PointCloud::has_color);
    let mut staged = prepare_staged(output_path, "0.las")?;
    let temp_path = staged.temp_path().to_path_buf();
    let create_writer = move |bounds| {
        get_las_writer(
            Some(guid.clone()),
            temp_path.clone(),
            bounds,
            has_color,
            las_version,
        )
        .context("Unable to create writer: ")
    };

    // ponytail: one writer thread encodes every point; parallel encoding plus
    // positioned writes into the file is the next step if it becomes the bottleneck.
    // The writer is a plain OS thread, not a rayon task: the pool may have a single
    // thread, which would deadlock if it blocked on `recv`.
    let (sender, receiver) = sync_channel(CHANNEL_CAPACITY);
    let staged_ref = &mut staged;
    let (decoded, written) = std::thread::scope(|scope| {
        let writer =
            scope.spawn(move || write_batches(receiver, has_color, staged_ref, create_writer));
        // Each task owns a clone of `sender`; all are dropped when the iteration
        // ends (also on error), which lets the writer finish. The file stays a
        // temporary until this function publishes it, so a decoder error does
        // not leave a LAS file behind.
        let decoded = pointclouds.par_iter().enumerate().try_for_each_with(
            sender,
            |sender, (index, pointcloud)| {
                println!("Saving pointcloud {index}...");
                send_pointcloud(index, input_path, pointcloud, sender)
                    .context(format!("Error while converting pointcloud {index}"))
            },
        );
        let written = writer
            .join()
            .unwrap_or_else(|_| Err(anyhow::anyhow!("The LAS writer thread panicked")));
        (decoded, written)
    });
    decoded?;

    // The writer thread has ended, so the partial file's handle can be released
    // and the temporary file removed before the fallback writes a new one.
    if !written.context("Error while writing the merged LAS file")? {
        discard_partial(staged)?;
        println!("{FALLBACK_MESSAGE}");
        return convert_pointclouds_two_pass(input_path, output_path, las_version);
    }
    staged.commit()
}

/// Two-pass fallback of [`convert_pointclouds`]: scans every cloud in parallel
/// for the merged bounds, then re-reads them sequentially to write the points
/// with a transform that fits them.
fn convert_pointclouds_two_pass(
    input_path: &Path,
    output_path: &Path,
    las_version: &LasVersion,
) -> Result<()> {
    let e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;
    let pointclouds = e57_reader.pointclouds();
    let guid = e57_reader.guid().to_owned();
    drop(e57_reader);

    let clouds = pointclouds
        .par_iter()
        .enumerate()
        .map(|(index, pointcloud)| -> Result<CloudStats> {
            println!("Saving pointcloud {index}...");

            let cloud = scan_pointcloud(input_path, pointcloud)
                .context(format!("Error while converting pointcloud {index}"))?;

            if cloud.skipped_points > 0 {
                println!(
                    "Pointcloud {index}: skipped {} points with invalid coordinates",
                    cloud.skipped_points
                );
            }

            Ok(cloud)
        })
        .collect::<Result<Vec<CloudStats>>>()
        .context("Error while converting pointcloud")?;

    let mut bounds = PointBounds::default();
    for cloud in &clouds {
        bounds.merge(&cloud.bounds);
    }
    let has_color = clouds.iter().any(|cloud| cloud.has_color);

    let mut staged = prepare_staged(output_path, "0.las")?;
    let temp_path = staged.temp_path().to_path_buf();
    staged.insert_writer(
        get_las_writer(Some(guid), temp_path, bounds, has_color, las_version)
            .context("Unable to create writer: ")?,
    )?;

    for (index, pointcloud) in pointclouds.iter().enumerate() {
        write_pointcloud(input_path, pointcloud, staged.writer_mut()?, has_color)
            .context(format!("Error while converting pointcloud {index}"))?;
    }

    staged.commit()
}

#[cfg(test)]
mod tests {
    use super::backfill_color;

    #[test]
    fn test_backfill_color_adds_default_when_format_has_color() {
        let mut point = las::Point::default();
        assert!(point.color.is_none());

        backfill_color(&mut point, true);

        assert_eq!(point.color, Some(las::Color::default()));
    }

    #[test]
    fn test_backfill_color_preserves_existing_color() {
        let existing = las::Color::new(1, 2, 3);
        let mut point = las::Point {
            color: Some(existing),
            ..Default::default()
        };

        backfill_color(&mut point, true);

        assert_eq!(point.color, Some(existing));
    }

    #[test]
    fn test_backfill_color_noop_when_format_has_no_color() {
        let mut point = las::Point::default();

        backfill_color(&mut point, false);

        assert!(point.color.is_none());
    }
}
