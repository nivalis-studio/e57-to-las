use std::ops::Deref;

use e57::{CartesianCoordinate, Point as E57Point};
use las::Point as LasPoint;

pub struct Point(E57Point);

impl Point {
    fn new(point: E57Point) -> Self {
        Self(point)
    }
}

impl Deref for Point {
    type Target = E57Point;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<E57Point> for Point {
    fn from(value: E57Point) -> Self {
        Self::new(value)
    }
}

impl From<Point> for Option<LasPoint> {
    fn from(value: Point) -> Self {
        let mut las_point = las::Point::default();

        if let CartesianCoordinate::Valid { x, y, z } = value.cartesian {
            las_point.x = x;
            las_point.y = y;
            las_point.z = z;
        } else {
            return None;
        }

        if let Some(ref color) = value.color {
            las_point.color = Some(las::Color {
                red: (color.red * u16::MAX as f32) as u16,
                green: (color.green * u16::MAX as f32) as u16,
                blue: (color.blue * u16::MAX as f32) as u16,
            })
        }

        if let Some(intensity) = value.intensity {
            las_point.intensity = (intensity * u16::MAX as f32) as u16;
        }

        Some(las_point)
    }
}
