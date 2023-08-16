use e57::Point as OriginalPoint;

use crate::utils::to_u16;

pub struct ExtendedPoint {
    pub original_point: OriginalPoint,
    pub rgb_color: las::Color,
}

impl From<OriginalPoint> for ExtendedPoint {
    fn from(point: OriginalPoint) -> Self {
        let rgb_color = if point.color_invalid == 0 {
            las::Color {
                red: to_u16(point.color.red),
                green: to_u16(point.color.green),
                blue: to_u16(point.color.blue),
            }
        } else {
            las::Color::default()
        };

        ExtendedPoint {
            original_point: point,
            rgb_color,
        }
    }
}
