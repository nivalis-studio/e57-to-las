use crate::colors::*;
use anyhow::Result;
use e57::Point;

pub fn point_converter(
    point: Point,
    color_limits: ParsedColorLimits,
    intensity_limits: ParsedIntensityLimits,
) -> Result<las::Point> {
    let las_colors = get_las_colors(&point.color, point.color_invalid, color_limits);

    let las_intensity =
        get_las_intensity(point.intensity, point.intensity_invalid, intensity_limits);

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
