use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::get_las_writer::get_las_writer;
use crate::{LasVersion, convert_point::convert_point, utils::create_path};

use anyhow::{Context, Result};
use e57::{E57Reader, PointCloud};

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
/// use e57_to_las::convert_pointcloud;
/// let pointcloud = e57::Pointcloud {  };
/// let input_path = String::from("path/to/input.e57");
/// let output_path = String::from("path/to/output");
/// convert_pointcloud(0, &pointcloud, input_path, output_path);
/// ```
pub fn convert_pointcloud(
    index: usize,
    pointcloud: &PointCloud,
    input_path: &String,
    output_path: &String,
    las_version: &LasVersion,
) -> Result<()> {
    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    let (mut max_x, mut max_y, mut max_z) =
        (f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut has_color = false;

    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;

        if point.color.is_some() {
            has_color = true;
        }

        let las_point = match convert_point(point) {
            Some(p) => p,
            None => continue,
        };

        max_x = max_x.max(las_point.x);
        max_y = max_y.max(las_point.y);
        max_z = max_z.max(las_point.z);
    }

    let max_cartesian = max_x.max(max_y).max(max_z);

    let path = create_path(
        Path::new(&output_path)
            .join("las")
            .join(format!("{}{}", index, ".las")),
    )
    .context("Unable to create path: ")?;

    let mut writer = get_las_writer(
        pointcloud.clone().guid,
        path,
        max_cartesian,
        has_color,
        las_version,
    )
    .context("Unable to create writer: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator for writing: ")?;

    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;

        if let Some(las_point) = convert_point(point) {
            writer.write_point(las_point).context("Unable to write: ")?;
        }
    }

    writer.close().context("Failed to close the writer: ")?;

    Ok(())
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
/// use e57_to_las::convert_pointclouds;
/// let e57_reader = e57::E57Reader::from_file("path/to/input.e56").context("Failed to open e57 file")?;
/// let output_path = String::from("path/to/output");
/// convert_pointclouds(e57_reader, output_path);
/// ```
pub fn convert_pointclouds(
    mut e57_reader: E57Reader<BufReader<File>>,
    output_path: &String,
    las_version: &LasVersion,
) -> Result<()> {
    let pointclouds = e57_reader.pointclouds();
    let guid = e57_reader.guid().to_owned();

    let mut max_cartesian = f64::NEG_INFINITY;
    let mut has_color = false;

    for (index, pointcloud) in pointclouds.iter().enumerate() {
        println!("Scanning pointcloud {} for bounds...", index);

        let pointcloud_reader = e57_reader
            .pointcloud_simple(pointcloud)
            .context("Unable to get point cloud iterator: ")?;

        let (mut max_x, mut max_y, mut max_z) =
            (f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);

        for p in pointcloud_reader {
            let point = p.context("Could not read point: ")?;

            if point.color.is_some() {
                has_color = true;
            }

            let las_point = match convert_point(point) {
                Some(p) => p,
                None => continue,
            };

            max_x = max_x.max(las_point.x);
            max_y = max_y.max(las_point.y);
            max_z = max_z.max(las_point.z);
        }

        max_cartesian = max_cartesian.max(max_x).max(max_y).max(max_z);
    }

    let path = create_path(
        Path::new(&output_path)
            .join("las")
            .join(format!("{}{}", 0, ".las")),
    )
    .context("Unable to create path: ")?;

    let mut writer = get_las_writer(Some(guid), path, max_cartesian, has_color, las_version)
        .context("Unable to create writer: ")?;

    for (index, pointcloud) in pointclouds.iter().enumerate() {
        println!("Writing pointcloud {}...", index);

        let pointcloud_reader = e57_reader
            .pointcloud_simple(pointcloud)
            .context("Unable to get point cloud iterator for writing: ")?;

        for p in pointcloud_reader {
            let point = p.context("Could not read point: ")?;

            if let Some(las_point) = convert_point(point) {
                writer.write_point(las_point).context("Unable to write: ")?;
            }
        }
    }

    writer.close().context("Failed to close the writer: ")?;
    Ok(())
}
