use e57::Point as OriginalPoint;

pub struct ExtendedPoint {
    pub original_point: OriginalPoint,
    pub rgb_color: Option<las::Color>,
}

impl From<OriginalPoint> for ExtendedPoint {
    fn from(point: OriginalPoint) -> Self {
        let rgb_color = if let (Some(col), Some(0), Some(intensity), Some(0)) = (
            &point.color,
            &point.color_invalid,
            &point.intensity,
            &point.intensity_invalid,
        ) {
            Some(las::Color {
                red: to_u16(col.red * intensity),
                green: to_u16(col.green * intensity),
                blue: to_u16(col.blue * intensity),
            })
        } else {
            None
        };

        ExtendedPoint {
            original_point: point,
            rgb_color,
        }
    }
}

fn clamp(value: f32) -> f32 {
    if value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        value
    }
}

fn to_u16(value: f32) -> u16 {
    // TODO: check if this is correct or if we should do * 255.0
    (clamp(value) * 65535.0) as u16
}
