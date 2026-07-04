use std::path::Path;

use crate::get_las_writer::{PointBounds, get_las_writer};
use crate::{LasVersion, convert_point::convert_point, utils::ensure_parent_dir};

use anyhow::{Context, Result};
use e57::{E57Reader, PointCloud};
use rayon::prelude::*;

/// The LAS points of a single E57 point cloud, along with the metadata
/// needed to configure a LAS writer for them.
struct CloudPoints {
    points: Vec<las::Point>,
    bounds: PointBounds,
    has_color: bool,
    skipped_points: usize,
}

/// Reads a single point cloud from an E57 file and converts its points to LAS points.
///
/// Opens its own `E57Reader` on `input_path` so that callers can safely invoke it
/// from parallel workers. Tracks the per-axis bounds of the converted points
/// (used to derive the LAS offset and scale), whether any point carries color,
/// and how many points were skipped because of invalid coordinates.
fn read_pointcloud(input_path: &Path, pointcloud: &PointCloud) -> Result<CloudPoints> {
    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    let mut points: Vec<las::Point> = Vec::new();
    let mut bounds = PointBounds::default();
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
        points.push(las_point);
    }

    Ok(CloudPoints {
        points,
        bounds,
        has_color,
        skipped_points,
    })
}

/// Converts a point cloud to a LAS file.
///
/// This function takes the points from the point cloud, converts them to LAS points using the
/// `convert_point` function, and writes them to `<output_path>/las/<index>.las`.
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
    let cloud = read_pointcloud(input_path, pointcloud)?;

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

    for mut p in cloud.points {
        backfill_color(&mut p, cloud.has_color);
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

/// Converts all point clouds of an E57 file to a single merged LAS file.
///
/// This function reads every point cloud of the E57 file at `input_path` in parallel
/// (each worker opens its own reader), converts the points to LAS points using the
/// `convert_point` function, and writes them all to `<output_path>/las/0.las`,
/// preserving the point cloud order.
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

    let clouds = pointclouds
        .par_iter()
        .enumerate()
        .map(|(index, pointcloud)| -> Result<CloudPoints> {
            println!("Saving pointcloud {index}...");

            let cloud = read_pointcloud(input_path, pointcloud)
                .context(format!("Error while converting pointcloud {index}"))?;

            if cloud.skipped_points > 0 {
                println!(
                    "Pointcloud {index}: skipped {} points with invalid coordinates",
                    cloud.skipped_points
                );
            }

            Ok(cloud)
        })
        .collect::<Result<Vec<CloudPoints>>>()
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

    for cloud in clouds {
        for mut p in cloud.points {
            backfill_color(&mut p, has_color);
            writer.write_point(p).context("Unable to write: ")?;
        }
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
