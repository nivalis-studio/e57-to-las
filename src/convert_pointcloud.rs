use crate::convert_point::convert_point;
use crate::get_las_writer::get_las_writer;
use crate::stations::{create_station_point, get_sum_coordinates, StationPoint};

use anyhow::{Context, Result};
use e57::{E57Reader, PointCloud};
use las::Write;
use std::collections::HashMap;

/// Converts a point cloud to a LAS file and returns a map of station points.
///
/// This function the points from the point cloud, converts them to LAS points using the `convert_point`
/// function, and writes them to the LAS file.
/// Additionally, it calculates the sum of coordinates and returns a hash map containing station
/// points created with `create_station_point`.
///
/// # Parameters
/// - `index`: The index of the point cloud.
/// - `pointcloud`: A reference to the point cloud to be converted.
/// - `input_path`: A reference to the input file path (E57 file).
/// - `output_path`: A reference to the output dir.
///
/// # Returns
/// - `Result<HashMap<usize, StationPoint>>`: A result containing a hash map that associates the index
///   with a station point. Returns an error if any part of the conversion fails, including if there
///   are no points in the point cloud.
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
) -> Result<HashMap<usize, StationPoint>> {
    let mut stations: HashMap<usize, StationPoint> = HashMap::new();

    let mut writer =
        get_las_writer(index, pointcloud, output_path).context("Unable to create writer: ")?;

    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    let mut count = 0.0;
    let mut sum_coordinates = (0.0, 0.0, 0.0);

    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;

        count += 1.0;
        sum_coordinates = get_sum_coordinates(sum_coordinates, &point);
        let las_point = match convert_point(point) {
            Some(p) => p,
            None => continue,
        };

        writer.write(las_point).context("Unable to write: ")?;
    }

    writer.close().context("Failed to close the writer: ")?;

    if count == 0.0 {
        return Err(anyhow::anyhow!("No points in pointcloud."));
    }

    stations.insert(index, create_station_point(sum_coordinates, count));

    Ok(stations)
}
