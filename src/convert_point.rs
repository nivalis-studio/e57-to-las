use crate::colors::*;
use anyhow::Result;
use e57::Point;

pub fn convert_point(
    point: Point,
    pointcloud_limits: ParsedPointCloudLimits,
) -> Result<las::Point> {
    let las_colors = get_las_colors(&point, pointcloud_limits.color_limits);
    let las_intensity = get_las_intensity(&point, pointcloud_limits.intensity_limits);

    let las_point = las::Point {
        x: point.cartesian.x,
        y: point.cartesian.y,
        z: point.cartesian.z,
        intensity: las_intensity,
        color: Some(las_colors),
        ..Default::default()
    };

    Ok(las_point)
}
