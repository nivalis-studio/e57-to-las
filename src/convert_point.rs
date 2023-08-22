use e57::CartesianCoordinate;

/// Converts an e57::Point to an optional las::Point.
///
/// This function takes an e57 point, extracts the Cartesian coordinates and optional color and intensity
/// attributes, and constructs a corresponding las point.
///
/// # Parameters
/// - `point`: The e57 point that needs to be converted.
///
/// # Returns
/// - `Option<las::Point>`: An optional las point. The function returns `None` if the Cartesian coordinates
///   are not valid. Otherwise, it returns a `Some(las::Point)` containing the converted point.
///
/// # Example
/// ```ignore
/// use e57_to_las::convert_point;
/// let e57_point = e57::Point {  };
/// let las_point = convert_point(e57_point);
/// ```
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
