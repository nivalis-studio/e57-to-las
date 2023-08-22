use std::collections::HashMap;

use crate::colors::get_pointcloud_limits;
use crate::stations::{create_station_point, get_sum_coordinates, StationPoint};
use crate::utils::construct_las_path;

use anyhow::{Context, Result};
use e57::{CartesianCoordinate, E57Reader, PointCloud};
use las::Write;
use uuid::Uuid;

pub fn convert_pointcloud(
    index: usize,
    pointcloud: &PointCloud,
    input_path: &String,
    output_path: &String,
) -> Result<HashMap<usize, StationPoint>> {
    let mut stations: HashMap<usize, StationPoint> = HashMap::new();
    let las_path =
        construct_las_path(output_path, index).context("Unable to create file path: ")?;

    let mut builder = las::Builder::from((1, 4));
    builder.point_format.has_color = true;
    builder.generating_software = String::from("e57_to_las");
    builder.guid =
        Uuid::parse_str(&pointcloud.guid.clone().replace("_", "-")).unwrap_or(Uuid::new_v4());

    let header = builder.into_header().context("Error encountered: ")?;

    let mut writer = las::Writer::from_path(&las_path, header).context("Error encountered: ")?;

    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let mut pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    let mut count = 0.0;
    let mut sum_coordinates = (0.0, 0.0, 0.0);

    let point_limits = get_pointcloud_limits(
        pointcloud.color_limits.clone(),
        pointcloud.intensity_limits.clone(),
    );

    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;

        count += 1.0;
        sum_coordinates = get_sum_coordinates(sum_coordinates, &point);
        let mut las_point = las::Point::default();

        if let CartesianCoordinate::Valid { x, y, z } = point.cartesian {
            las_point.x = x;
            las_point.y = y;
            las_point.z = z;
        } else {
            continue;
        }
        if let Some(color) = point.color {
            las_point.color = Some(las::Color {
                red: (color.red * u16::MAX as f32) as u16,
                green: (color.green * u16::MAX as f32) as u16,
                blue: (color.blue * u16::MAX as f32) as u16,
            })
        }
        if let Some(intensity) = point.intensity {
            las_point.intensity = (intensity * u16::MAX as f32) as u16;
        }

        writer.write(las_point).context("Unable to write: ")?;
    }

    writer.close().context("Failed to close the writer: ")?;

    if count == 0.0 {
        return Err(anyhow::anyhow!("No points in pointcloud."));
    }

    stations.insert(index, create_station_point(sum_coordinates, count));

    Ok(stations)
}
