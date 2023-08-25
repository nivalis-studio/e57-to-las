use crate::convert_point::convert_point;
use crate::get_las_writer::get_las_writer;

use anyhow::{Context, Result};
use e57::{E57Reader, PointCloud};
use las::Write;

/// Converts a point cloud to a LAS file and returns a map of station points.
///
/// This function the points from the point cloud, converts them to LAS points using the `convert_point`
/// function, and writes them to the LAS file.
/// Additionally, it returns a hash map containing the station points,
/// the coords in the local spatial system of the pointcloud.
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
) -> Result<()> {
    let mut writer =
        get_las_writer(index, pointcloud, output_path).context("Unable to create writer: ")?;

    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;

        let las_point = match convert_point(point) {
            Some(p) => p,
            None => continue,
        };

        writer.write(las_point).context("Unable to write: ")?;
    }

    writer.close().context("Failed to close the writer: ")?;

    Ok(())
}
