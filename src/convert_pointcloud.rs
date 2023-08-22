use crate::colors::get_pointcloud_limits;
use crate::convert_point::convert_point;
use crate::stations::{create_station_point, get_sum_coordinates, StationPoint};
use crate::utils::construct_las_path;

use anyhow::{Context, Result};
use e57::{E57Reader, PointCloud};
use las::Write;
use uuid::Uuid;

pub fn convert_pointcloud(
    index: usize,
    pointcloud: &PointCloud,
    input_path: &String,
    output_path: &String,
) -> Result<Vec<StationPoint>> {
    let mut stations: Vec<StationPoint> = Vec::new();
    let las_path =
        construct_las_path(output_path, index).context("Unable to create file path: ")?;

    let mut builder = las::Builder::from((1, 4));
    builder.point_format.has_color = true;
    builder.generating_software = String::from("e57_to_las");
    builder.guid = Uuid::parse_str(&pointcloud.guid.clone().replace("_", "-"))
        .context("Invalid pointcloud guid: ")?;

    let header = builder.into_header().context("Error encountered: ")?;

    let mut writer = las::Writer::from_path(&las_path, header).context("Error encountered: ")?;

    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let mut pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;
    pointcloud_reader.skip_invalid(true);

    let mut count = 0.0;
    let mut sum_coordinates = (0.0, 0.0, 0.0);

    let point_limits = get_pointcloud_limits(
        pointcloud.color_limits.clone(),
        pointcloud.intensity_limits.clone(),
    );

    pointcloud_reader.for_each(|p| {
        let point = match p {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Could not read point: {}", e);
                return ();
            }
        };

        count += 1.0;
        sum_coordinates = get_sum_coordinates(sum_coordinates, &point);

        let las_point = match convert_point(point, point_limits.clone()) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Could not convert point: {}", e);
                return ();
            }
        };

        match writer.write(las_point) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Unable to write: {}", e);
                return ();
            }
        };
    });

    writer.close().context("Failed to close the writer: ")?;

    if count == 0.0 {
        return Err(anyhow::anyhow!("No points in pointcloud."));
    }

    stations.push(create_station_point(sum_coordinates, count));

    Ok(stations)
}
