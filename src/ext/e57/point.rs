#[derive(Clone, Copy)]
pub struct PointMeta {
    pub point_format: las::point::Format,
    pub gps_time: Option<f64>,
}

impl PointMeta {
    pub fn new(point_format: las::point::Format, pc: &e57::PointCloud) -> Self {
        Self {
            point_format,
            gps_time: point_format.has_gps_time.then(|| {
                pc.acquisition_start
                    .as_ref()
                    .map(|d| d.gps_time)
                    .unwrap_or_default()
            }),
        }
    }
}

pub trait E57PointExt {
    fn to_las_point(self, meta: &PointMeta) -> las::Point;
}

impl E57PointExt for e57::Point {
    #[inline]
    fn to_las_point(self, meta: &PointMeta) -> las::Point {
        let mut las_point = las::Point::default();

        if let e57::CartesianCoordinate::Valid { x, y, z } = self.cartesian {
            las_point.x = x;
            las_point.y = y;
            las_point.z = z;
        }

        if meta.point_format.has_color {
            las_point.color = Some(self.color.map_or(BLACK, |c| las::Color {
                red: f01_to_u16(c.red),
                green: f01_to_u16(c.green),
                blue: f01_to_u16(c.blue),
            }));
        }

        if let Some(i) = self.intensity {
            las_point.intensity = f01_to_u16(i);
        }

        las_point.gps_time = meta.gps_time;

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
