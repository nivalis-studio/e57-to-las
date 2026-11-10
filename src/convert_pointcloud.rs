use std::{fs::File, io::BufWriter, path::Path};

use crate::get_las_writer::{PointBounds, get_las_writer};
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

/// Pass 1: scans a single point cloud without keeping its points.
///
/// Opens its own `E57Reader` on `input_path` so that callers can safely invoke it
/// from parallel workers. Tracks the per-axis bounds of the converted points
/// (used to derive the LAS offset and scale), whether any point carries color,
/// and how many points were skipped because of invalid coordinates.
fn scan_pointcloud(input_path: &Path, pointcloud: &PointCloud) -> Result<CloudStats> {
    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    let mut stats = CloudStats {
        bounds: PointBounds::default(),
        has_color: false,
        skipped_points: 0,
    };

    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;

        if point.color.is_some() {
            stats.has_color = true;
        }

        match convert_point(point) {
            Some(las_point) => stats.bounds.update(&las_point),
            None => stats.skipped_points += 1,
        }
    }

    Ok(stats)
}

/// Pass 2: re-reads a single point cloud and streams its points to `writer`.
///
/// Points rejected by `convert_point` are skipped; [`scan_pointcloud`] applies
/// the same rule, so they were already counted there.
fn write_pointcloud(
    input_path: &Path,
    pointcloud: &PointCloud,
    writer: &mut las::Writer<BufWriter<File>>,
    has_color: bool,
) -> Result<()> {
    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;

        if let Some(mut las_point) = convert_point(point) {
            backfill_color(&mut las_point, has_color);
            writer.write_point(las_point).context("Unable to write: ")?;
        }
    }

    Ok(())
}

type LasWriter = las::Writer<BufWriter<File>>;

/// One pass: decodes `pointcloud` once and streams its valid points to `writer`.
///
/// If `writer` is still `None` at the first valid point, it is created with
/// `create_writer`, given bounds holding only that point: the LAS offset is
/// that point rounded to meters and the scale is 0.001. Every point is then
/// checked against the writer's own transform.
///
/// Returns the number of points skipped because of invalid coordinates, or
/// `None` if a point lies too far from the offset to fit in i32 at that scale
/// (more than ~2,147 km). The writer is then closed and the caller must redo
/// the conversion with the two-pass functions, which recreate (truncate) the file.
fn stream_pointcloud(
    input_path: &Path,
    pointcloud: &PointCloud,
    writer: &mut Option<LasWriter>,
    has_color: bool,
    create_writer: &mut impl FnMut(PointBounds) -> Result<LasWriter>,
) -> Result<Option<usize>> {
    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    let mut skipped_points = 0;
    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;

        let Some(mut las_point) = convert_point(point) else {
            skipped_points += 1;
            continue;
        };

        let writer = match writer {
            Some(writer) => writer,
            None => {
                let mut bounds = PointBounds::default();
                bounds.update(&las_point);
                writer.insert(create_writer(bounds)?)
            }
        };

        let t = writer.header().transforms();
        let fits = t.x.inverse(las_point.x).is_ok()
            && t.y.inverse(las_point.y).is_ok()
            && t.z.inverse(las_point.z).is_ok();
        if !fits {
            writer.close().context("Failed to close the writer: ")?;
            return Ok(None);
        }

        backfill_color(&mut las_point, has_color);
        writer.write_point(las_point).context("Unable to write: ")?;
    }

    Ok(Some(skipped_points))
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
    let path = ensure_parent_dir(output_path.join("las").join(format!("{index}.las")))
        .context("Unable to create path: ")?;
    let mut create_writer = |bounds| {
        get_las_writer(
            pointcloud.guid.clone(),
            path.clone(),
            bounds,
            has_color,
            las_version,
        )
        .context("Unable to create writer: ")
    };

    let mut writer = None;
    let Some(skipped_points) = stream_pointcloud(
        input_path,
        pointcloud,
        &mut writer,
        has_color,
        &mut create_writer,
    )?
    else {
        // Release the partial file's handle before the fallback truncates it.
        drop(writer);
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
    let mut writer = match writer {
        Some(writer) => writer,
        None => create_writer(PointBounds::default())?,
    };
    writer.close().context("Failed to close the writer: ")?;

    Ok(())
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

    let path = ensure_parent_dir(output_path.join("las").join(format!("{index}.las")))
        .context("Unable to create path: ")?;

    let mut writer = get_las_writer(
        pointcloud.guid.clone(),
        path,
        cloud.bounds,
        cloud.has_color,
        las_version,
    )
    .context("Unable to create writer: ")?;

    write_pointcloud(input_path, pointcloud, &mut writer, cloud.has_color)?;

    writer.close().context("Failed to close the writer: ")?;

    Ok(())
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
/// sequentially in index order, converts the points to LAS points using the
/// `convert_point` function, and streams them to `<output_path>/las/0.las`.
/// No points are buffered in memory. The LAS offset is the first valid point
/// (rounded to meters), the scale is 0.001 and the point format has color if any
/// E57 prototype declares color. If a point lies more than ~2,147 km from that
/// offset, the whole file is rewritten in two passes with a coarser scale.
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
    let path = ensure_parent_dir(output_path.join("las").join("0.las"))
        .context("Unable to create path: ")?;
    let mut create_writer = |bounds| {
        get_las_writer(
            Some(guid.clone()),
            path.clone(),
            bounds,
            has_color,
            las_version,
        )
        .context("Unable to create writer: ")
    };

    // ponytail: clouds are decoded sequentially into the single writer; a bounded
    // channel fed by parallel decoders is the upgrade if merged wall time matters.
    let mut writer = None;
    for (index, pointcloud) in pointclouds.iter().enumerate() {
        println!("Saving pointcloud {index}...");

        let Some(skipped_points) = stream_pointcloud(
            input_path,
            pointcloud,
            &mut writer,
            has_color,
            &mut create_writer,
        )
        .context(format!("Error while converting pointcloud {index}"))?
        else {
            // Release the partial file's handle before the fallback truncates it.
            drop(writer);
            println!("{FALLBACK_MESSAGE}");
            return convert_pointclouds_two_pass(input_path, output_path, las_version);
        };

        if skipped_points > 0 {
            println!(
                "Pointcloud {index}: skipped {skipped_points} points with invalid coordinates"
            );
        }
    }

    // No valid point: still write an (empty) LAS file.
    let mut writer = match writer {
        Some(writer) => writer,
        None => create_writer(PointBounds::default())?,
    };
    writer.close().context("Failed to close the writer: ")?;
    Ok(())
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

    let path = ensure_parent_dir(output_path.join("las").join("0.las"))
        .context("Unable to create path: ")?;

    let mut writer = get_las_writer(Some(guid), path, bounds, has_color, las_version)
        .context("Unable to create writer: ")?;

    for (index, pointcloud) in pointclouds.iter().enumerate() {
        write_pointcloud(input_path, pointcloud, &mut writer, has_color)
            .context(format!("Error while converting pointcloud {index}"))?;
    }

    writer.close().context("Failed to close the writer: ")?;
    Ok(())
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
