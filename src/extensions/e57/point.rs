pub trait E57PointExt {
    fn to_las_point(self, point_format: las::point::Format) -> las::Point;
}

impl E57PointExt for e57::Point {
    fn to_las_point(self, point_format: las::point::Format) -> las::Point {
        let mut las_point = las::Point::default();

        if let e57::CartesianCoordinate::Valid { x, y, z } = self.cartesian {
            las_point.x = x;
            las_point.y = y;
            las_point.z = z;
        }

        if point_format.has_color {
            las_point.color = Some(match self.color {
                Some(c) => las::Color {
                    red: f01_to_u16(c.red),
                    green: f01_to_u16(c.green),
                    blue: f01_to_u16(c.blue),
                },
                None => BLACK,
            });
        }

        if let Some(i) = self.intensity {
            las_point.intensity = f01_to_u16(i);
        }

        las_point
    }
}

#[inline(always)]
fn f01_to_u16(x: f32) -> u16 {
    if !x.is_finite() {
        return 0;
    }
    (x.clamp(0.0, 1.0) * 65535.0 + 0.5).floor() as u16
}

const BLACK: las::Color = las::Color {
    red: 0,
    green: 0,
    blue: 0,
};
