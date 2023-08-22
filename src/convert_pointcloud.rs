use crate::convert_point::convert_point;
use crate::get_las_writer::get_las_writer;
use crate::stations::{create_station_point, get_sum_coordinates, StationPoint};

use anyhow::{Context, Result};
use e57::{E57Reader, PointCloud};
use las::Write;
use std::collections::HashMap;

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
