use e57::CartesianCoordinate;

pub trait E57PointExt {
    fn x_to_las(&self) -> Option<las::Point>;
}

impl E57PointExt for e57::Point {
    fn x_to_las(&self) -> Option<las::Point> {
        let mut las_point = las::Point::default();

        if let CartesianCoordinate::Valid { x, y, z } = self.cartesian {
            las_point.x = x;
            las_point.y = y;
            las_point.z = z;
        } else {
            return None;
        }

        if let Some(ref color) = self.color {
            las_point.color = Some(las::Color {
                red: (color.red * u16::MAX as f32) as u16,
                green: (color.green * u16::MAX as f32) as u16,
                blue: (color.blue * u16::MAX as f32) as u16,
            })
        }

        if let Some(intensity) = self.intensity {
            las_point.intensity = (intensity * u16::MAX as f32) as u16;
        }

        Some(las_point)
    }
}
