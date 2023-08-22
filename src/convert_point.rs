use e57::CartesianCoordinate;

pub fn convert_point(point: e57::Point) -> Option<las::Point> {
    let mut las_point = las::Point::default();

    if let CartesianCoordinate::Valid { x, y, z } = point.cartesian {
        las_point.x = x;
        las_point.y = y;
        las_point.z = z;
    } else {
        return None;
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

    Some(las_point)
}
