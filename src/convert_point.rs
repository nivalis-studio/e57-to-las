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
///   are not valid, or if `x`, `y` or `z` is not finite (`NaN` or ±infinity). Otherwise, it returns a
///   `Some(las::Point)` containing the converted point.
///
/// # Example
/// ```ignore
/// use e57_to_las::convert_point;
/// let e57_point = e57::Point {  };
/// let las_point = convert_point(e57_point);
/// ```
pub fn convert_point(point: e57::Point) -> Option<las::Point> {
    let mut las_point = las::Point::default();

    let CartesianCoordinate::Valid { x, y, z } = point.cartesian else {
        return None;
    };
    // The E57 reader reports these as valid. NaN lands on the LAS offset and
    // ±infinity makes the header scale infinite, so every point reads back as NaN.
    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return None;
    }
    las_point.x = x;
    las_point.y = y;
    las_point.z = z;

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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::convert_point;
    use e57::{CartesianCoordinate, Color, Point, SphericalCoordinate};

    fn e57_point(cartesian: CartesianCoordinate) -> Point {
        Point {
            cartesian,
            spherical: SphericalCoordinate::Invalid,
            color: None,
            intensity: None,
            row: -1,
            column: -1,
        }
    }

    fn valid() -> CartesianCoordinate {
        CartesianCoordinate::Valid {
            x: 1.25,
            y: -2.5,
            z: 1e6 + 0.125,
        }
    }

    #[test]
    fn test_invalid_cartesian_is_skipped() {
        assert!(convert_point(e57_point(CartesianCoordinate::Invalid)).is_none());
    }

    #[test]
    fn test_direction_cartesian_is_skipped() {
        let direction = CartesianCoordinate::Direction {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        assert!(convert_point(e57_point(direction)).is_none());
    }

    #[test]
    fn test_non_finite_coordinates_are_skipped() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let cases = [
                CartesianCoordinate::Valid {
                    x: bad,
                    y: 1.0,
                    z: 2.0,
                },
                CartesianCoordinate::Valid {
                    x: 1.0,
                    y: bad,
                    z: 2.0,
                },
                CartesianCoordinate::Valid {
                    x: 1.0,
                    y: 2.0,
                    z: bad,
                },
            ];
            for cartesian in cases {
                assert!(
                    convert_point(e57_point(cartesian.clone())).is_none(),
                    "non-finite coordinate should be skipped: {cartesian:?}"
                );
            }
        }
    }

    #[test]
    fn test_valid_coordinates_are_copied_exactly() {
        let p = convert_point(e57_point(valid())).expect("valid point");
        assert_eq!((p.x, p.y, p.z), (1.25, -2.5, 1e6 + 0.125));
    }

    #[test]
    fn test_color_is_scaled_to_u16() {
        let mut point = e57_point(valid());
        point.color = Some(Color {
            red: 1.0,
            green: 0.0,
            blue: 0.5,
        });
        let p = convert_point(point).expect("valid point");
        // 0.5 * 65535 = 32767.5, truncated by the `as u16` cast.
        assert_eq!(p.color, Some(las::Color::new(65535, 0, 32767)));
    }

    #[test]
    fn test_missing_color_stays_none() {
        let p = convert_point(e57_point(valid())).expect("valid point");
        assert_eq!(p.color, None);
    }

    #[test]
    fn test_intensity_is_scaled_to_u16() {
        let mut point = e57_point(valid());
        point.intensity = Some(1.0);
        let p = convert_point(point).expect("valid point");
        assert_eq!(p.intensity, 65535);
    }

    #[test]
    fn test_missing_intensity_is_zero() {
        let p = convert_point(e57_point(valid())).expect("valid point");
        assert_eq!(p.intensity, 0);
    }
}
