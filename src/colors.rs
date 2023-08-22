use e57::{ColorLimits, IntensityLimits};
use e57::{RecordDataType, RecordValue};

#[derive(Clone, Debug)]
pub struct ParsedColorLimits {
    red_min: f32,
    red_max: f32,
    green_min: f32,
    green_max: f32,
    blue_min: f32,
    blue_max: f32,
}

impl Default for ParsedColorLimits {
    fn default() -> Self {
        Self {
            red_min: 0.0,
            red_max: 255.0,
            green_min: 0.0,
            green_max: 255.0,
            blue_min: 0.0,
            blue_max: 255.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParsedIntensityLimits {
    intensity_min: f32,
    intensity_max: f32,
}

impl Default for ParsedIntensityLimits {
    fn default() -> Self {
        Self {
            intensity_min: 0.0,
            intensity_max: 255.0,
        }
    }
}

fn get_max(value: Option<RecordValue>) -> f32 {
    value.map_or(255.0, |v| {
        v.to_unit_f32(&RecordDataType::Single {
            min: None,
            max: None,
        })
        .unwrap_or(255.0)
    })
}

fn get_min(value: Option<RecordValue>) -> f32 {
    value.map_or(0.0, |v| {
        v.to_unit_f32(&RecordDataType::Single {
            min: None,
            max: None,
        })
        .unwrap_or(0.0)
    })
}

pub fn to_u16(value: f32, min_limit: f32, max_limit: f32) -> u16 {
    if max_limit <= min_limit {
        // TODO: add implementation for min_limit
        dbg!("max_limit must be greater than min_limit");
    }
    (value.clamp(0.0, 1.0) * max_limit) as u16
}

pub fn get_colors_limit(color_limits: Option<ColorLimits>) -> ParsedColorLimits {
    let red_min = get_min(color_limits.as_ref().and_then(|cl| cl.red_min.clone()));
    let red_max = get_max(color_limits.as_ref().and_then(|cl| cl.red_max.clone()));
    let green_min = get_min(color_limits.as_ref().and_then(|cl| cl.green_min.clone()));
    let green_max = get_max(color_limits.as_ref().and_then(|cl| cl.green_max.clone()));
    let blue_min = get_min(color_limits.as_ref().and_then(|cl| cl.blue_min.clone()));
    let blue_max = get_max(color_limits.as_ref().and_then(|cl| cl.blue_max.clone()));

    ParsedColorLimits {
        red_min,
        red_max,
        green_min,
        green_max,
        blue_min,
        blue_max,
    }
}

pub fn get_intensity_limits(intensity_limits: Option<IntensityLimits>) -> ParsedIntensityLimits {
    let intensity_min = intensity_limits
        .as_ref()
        .and_then(|il| il.intensity_min.clone())
        .map_or(0.0, |im| get_min(Some(im)));
    let intensity_max = intensity_limits
        .as_ref()
        .and_then(|il| il.intensity_max.clone())
        .map_or(255.0, |im| get_max(Some(im)));

    ParsedIntensityLimits {
        intensity_min,
        intensity_max,
    }
}

pub fn get_las_colors(
    color: &e57::Color,
    color_invalid: u8,
    color_limits: ParsedColorLimits,
) -> las::Color {
    let las_colors = if color_invalid == 0 {
        las::Color {
            red: to_u16(color.red, color_limits.red_min, color_limits.red_max),
            green: to_u16(color.green, color_limits.green_min, color_limits.green_max),
            blue: to_u16(color.blue, color_limits.blue_min, color_limits.blue_max),
        }
    } else {
        las::Color::default()
    };

    las_colors
}

pub fn get_las_intensity(
    intensity: f32,
    intensity_invalid: u8,
    intensity_limits: ParsedIntensityLimits,
) -> u16 {
    // A value of zero means the intensity is valid, 1 means invalid.
    if intensity_invalid == 1 {
        return 0;
    }

    to_u16(
        intensity,
        intensity_limits.intensity_min,
        intensity_limits.intensity_max,
    )
}
