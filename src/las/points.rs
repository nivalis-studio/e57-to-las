use uuid::Uuid;

/// An extension trait for `Vec<las::Point>`, adding utilities for working with collections of LAS points.
pub trait LasPointsExt {
    /// Checks if any point in the collection contains color data.
    ///
    /// This method iterates through the points and determines if any of them contain color information.
    /// How the check is performed can vary by implementation, but it typically looks at whether
    /// the `color` attribute of the points is populated.
    ///
    /// # Returns
    /// - `true` if at least one point has color data.
    /// - `false` otherwise.
    ///
    /// # Example
    /// ```
    /// use las::{Point, Color};
    /// use e57_to_las::las::LasPointsExt;
    ///
    /// let points = vec![
    ///     Point { color: None, ..Default::default() },
    ///     Point { color: Some(Color { red: 255, green: 0, blue: 0 }), ..Default::default() }
    /// ];
    ///
    /// assert!(points.x_has_color());
    /// ```
    fn x_has_color(&self) -> bool;

    /// Finds the maximum Cartesian coordinate value (x, y, or z) in the point collection.
    ///
    /// This method calculates the maximum value across the Cartesian coordinates (x, y, z) of the points.
    /// The exact calculation logic can be implemented as needed, typically to find the highest coordinate in any dimension.
    ///
    /// # Returns
    /// The highest value found in the x, y, or z coordinates of the points.
    ///
    /// # Example
    /// ```
    /// use las::Point;
    /// use e57_to_las::las::LasPointsExt;
    ///
    /// let points = vec![
    ///     Point { x: 1.0, y: 2.0, z: 3.0, ..Default::default() },
    ///     Point { x: 4.0, y: 5.0, z: 6.0, ..Default::default() }
    /// ];
    ///
    /// assert_eq!(points.x_max_cartesian(), 6.0);
    /// ```
    fn x_max_cartesian(&self) -> f64;

    /// Creates a `las::Builder` based on the point collection.
    ///
    /// The method is expected to generate a builder configured with relevant metadata from the points, such as
    /// whether the point format includes color information and appropriate coordinate scaling. The exact details
    /// of how the builder is populated can vary by implementation.
    ///
    /// # Returns
    /// A `las::Builder` initialized based on the point collection.
    ///
    /// # Example
    /// ```
    /// use las::Point;
    /// use e57_to_las::las::LasPointsExt;
    ///
    /// let points = vec![
    ///     Point { x: 1.0, y: 2.0, z: 3.0, ..Default::default() }
    /// ];
    ///
    /// let builder = points.x_header_builder();
    /// assert!(builder.point_format.has_color == false);
    /// ```
    fn x_header_builder(&self) -> las::Builder;
}

impl LasPointsExt for Vec<las::Point> {
    /// Checks if any point in the collection contains color data.
    ///
    /// This implementation returns `true` if at least one `las::Point` in the collection has non-`None` color data.
    fn x_has_color(&self) -> bool {
        self.iter().any(|p| p.color.is_some())
    }

    /// Finds the maximum Cartesian coordinate value (x, y, or z) in the point collection.
    ///
    /// This implementation calculates the largest `x`, `y`, or `z` coordinate across all points in the collection.
    fn x_max_cartesian(&self) -> f64 {
        self.iter()
            .fold(f64::NEG_INFINITY, |c, p| c.max(p.x).max(p.y).max(p.z))
    }

    /// Builds a `las::Builder` using the points' metadata.
    ///
    /// The `las::Builder` is configured with a new `GUID`, whether the point format includes color, and
    /// the smallest scale based on the maximum Cartesian coordinate values in the point collection.
    fn x_header_builder(&self) -> las::Builder {
        let mut builder = las::Builder::default();

        builder.point_format.has_color = self.x_has_color();
        builder.guid = Uuid::new_v4();

        let offset = 0.0;
        let scale = find_smallest_scale(self.x_max_cartesian());
        let transform = las::Transform { scale, offset };
        builder.transforms = las::Vector {
            x: transform,
            y: transform,
            z: transform,
        };

        builder
    }
}

/// Helper function to calculate the smallest scale that can accurately represent the maximum coordinate value.
fn find_smallest_scale(x: f64) -> f64 {
    let mut scale = 0.001;
    let min_i32 = f64::from(i32::MIN);
    let max_i32 = f64::from(i32::MAX);

    while (x / scale).round() < min_i32 || (x / scale).round() > max_i32 {
        scale += 0.0001;
    }

    scale
}
