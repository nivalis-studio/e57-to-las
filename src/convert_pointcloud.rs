use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::get_las_writer::{PointBounds, get_las_writer};
use crate::{LasVersion, convert_point::convert_point, utils::create_path};

use anyhow::{Context, Result};
use e57::{E57Reader, PointCloud};
use rayon::prelude::*;

/// Converts a point cloud to a LAS file.
///
/// This function take the points from the point cloud, converts them to LAS points using the `convert_point`
/// function, and writes them in a LAS file.
///
/// # Parameters
/// - `index`: The index of the point cloud.
/// - `pointcloud`: A reference to the point cloud to be converted.
/// - `input_path`: A reference to the input file path (E57 file).
/// - `output_path`: A reference to the output dir.
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
    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    let mut bounds = PointBounds::default();

    let mut las_points: Vec<las::Point> = Vec::new();
    let mut has_color = false;
    let mut skipped_points: usize = 0;

    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;

        if point.color.is_some() {
            has_color = true;
        }

        let las_point = match convert_point(point) {
            Some(p) => p,
            None => {
                skipped_points += 1;
                continue;
            }
        };

        bounds.update(&las_point);
        las_points.push(las_point);
    }

    if skipped_points > 0 {
        println!("Pointcloud {index}: skipped {skipped_points} points with invalid coordinates");
    }

    let path = create_path(output_path.join("las").join(format!("{}{}", index, ".las")))
        .context("Unable to create path: ")?;

    let mut writer = get_las_writer(
        pointcloud.clone().guid,
        path,
        bounds,
        has_color,
        las_version,
    )
    .context("Unable to create writer: ")?;

    for mut p in las_points {
        backfill_color(&mut p, has_color);
        writer.write_point(p).context("Unable to write: ")?;
    }

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

/// Converts the pointclouds of an E57Reader to a single LAS file.
///
/// This function takes a E57Reader, reads its pointclouds, converts them to LAS points using the `convert_point`
/// function, and writes them in a single LAS file.
///
/// # Parameters
/// - `e57_reader`: A E57Reader from a file.
/// - `output_path`: A reference to the output dir.
///
/// # Example
/// ```ignore
/// use std::path::Path;
/// use e57_to_las::{convert_pointclouds, LasVersion};
/// use anyhow::Context;
///
/// # fn example() -> anyhow::Result<()> {
/// let input_path = Path::new("path/to/input.e57");
/// let e57_reader = e57::E57Reader::from_file(input_path).context("Failed to open e57 file")?;
/// let output_path = Path::new("path/to/output");
/// let las_version = LasVersion::new(1, 4)?;
/// convert_pointclouds(e57_reader, output_path, &las_version)?;
/// # Ok(())
/// # }
/// ```
pub fn convert_pointclouds(
    e57_reader: E57Reader<BufReader<File>>,
    output_path: &Path,
    las_version: &LasVersion,
) -> Result<()> {
    let pointclouds = e57_reader.pointclouds();
    let guid = &e57_reader.guid().to_owned();
    let e57_reader_mutex = Mutex::new(e57_reader);

    let bounds_mutex = Mutex::new(PointBounds::default());
    let las_points: Vec<las::Point> = Vec::new();
    let las_points_mutex = Mutex::new(las_points);
    let has_color = Arc::new(AtomicBool::new(false));

    pointclouds
        .par_iter()
        .enumerate()
        .try_for_each(|(index, pointcloud)| -> Result<()> {
            println!("Saving pointclouds {index}...");

            let mut reader = e57_reader_mutex.lock().unwrap_or_else(|e| e.into_inner());
            let pointcloud_reader = reader
                .pointcloud_simple(pointcloud)
                .context("Unable to get point cloud iterator: ")?;

            let mut local_bounds = PointBounds::default();
            let mut skipped_points: usize = 0;
            for p in pointcloud_reader {
                let point = p.context("Could not read point: ")?;

                if point.color.is_some() {
                    has_color.store(true, Ordering::Relaxed);
                }

                let las_point = match convert_point(point) {
                    Some(p) => p,
                    None => {
                        skipped_points += 1;
                        continue;
                    }
                };

                local_bounds.update(&las_point);
                las_points_mutex
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(las_point);
            }

            if skipped_points > 0 {
                println!(
                    "Pointcloud {index}: skipped {skipped_points} points with invalid coordinates"
                );
            }

            bounds_mutex
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .merge(&local_bounds);

            Ok(())
        })
        .context("Error while converting pointcloud")?;

    let path = create_path(output_path.join("las").join(format!("{}{}", 0, ".las")))
        .context("Unable to create path: ")?;

    let bounds = *bounds_mutex.lock().unwrap_or_else(|e| e.into_inner());

    let mut writer = get_las_writer(
        Some(guid.to_owned()),
        path,
        bounds,
        has_color.load(Ordering::Relaxed),
        las_version,
    )
    .context("Unable to create writer: ")?;

    let las_points = las_points_mutex
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();

    let has_color = has_color.load(Ordering::Relaxed);

    for mut p in las_points {
        backfill_color(&mut p, has_color);
        writer.write_point(p).context("Unable to write: ")?;
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
