use e57::CartesianCoordinate;

/// An extension trait for the `e57::Point` type, adding functionality to convert it to a `las::Point`.
pub trait E57PointExt {
    /// Converts an `e57::Point` to a `las::Point`, typically by extracting Cartesian coordinates and other attributes.
    ///
    /// The exact conversion logic can be customized by different implementations of this trait, but the method is
    /// generally expected to return a valid `las::Point` if the `e57::Point` contains valid data.
    ///
    /// # Returns
    /// - `Some(las::Point)` when the conversion is successful.
    /// - `None` if the conversion fails, for example, due to invalid or missing data in the `e57::Point`.
    fn x_to_las(&self) -> Option<las::Point>;
}

impl E57PointExt for e57::Point {
    /// Converts the `e57::Point` to a `las::Point`.
    ///
    /// This implementation extracts the Cartesian `x`, `y`, and `z` coordinates and optional attributes like
    /// `color` and `intensity` from the `e57::Point`. It returns `None` if the Cartesian coordinates are invalid.
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
