use crate::colors::get_pointcloud_limits;
use crate::convert_point::convert_point;
use crate::stations::{create_station_point, get_sum_coordinate, StationPoint};
use crate::utils::construct_las_path;

use anyhow::Result;
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
    let las_path = match construct_las_path(output_path, index) {
        Ok(p) => p,
        Err(e) => {
            return Err(anyhow::anyhow!("Unable to create file path: {}", e));
        }
    };

    let mut builder = las::Builder::from((1, 4));
    builder.point_format.has_color = true;
    builder.generating_software = String::from("e57_to_las");
    builder.guid = match Uuid::parse_str(&pointcloud.guid.clone().replace("_", "-")) {
        Ok(g) => g,
        Err(e) => {
            return Err(anyhow::anyhow!("Invalid guid: {}", e));
        }
    };

    let header = match builder.into_header() {
        Ok(h) => h,
        Err(e) => {
            return Err(anyhow::anyhow!("Error encountered: {}", e));
        }
    };

    let mut writer = match las::Writer::from_path(&las_path, header) {
        Ok(w) => w,
        Err(e) => {
            return Err(anyhow::anyhow!("Error encountered: {}", e));
        }
    };

    let mut e57_reader = match E57Reader::from_file(input_path) {
        Ok(r) => r,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to open e57 file: {}", e));
        }
    };

    let mut pointcloud_reader = match e57_reader.pointcloud_simple(pointcloud) {
        Ok(i) => i,
        Err(e) => {
            return Err(anyhow::anyhow!("Unable to get point cloud iterator: {}", e));
        }
    };

    pointcloud_reader.skip_invalid(true);

    let mut count = 0.0;
    let mut sum_coordinate = (0.0, 0.0, 0.0);

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
        sum_coordinate = get_sum_coordinate(sum_coordinate, &point);

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

    match writer.close() {
        Ok(_) => (),
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to close the writer: {}", e));
        }
    };

    if count == 0.0 {
        return Err(anyhow::anyhow!("No points in pointcloud."));
    }

    stations.push(create_station_point(sum_coordinate, count));

    Ok(stations)
}
