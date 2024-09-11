use super::E57PointExt;
use e57::PointCloudReaderSimple;
use std::io::{Read, Seek};

/// An extension trait for the `e57::PointCloudReaderSimple` type, allowing point clouds to be converted to LAS format.
pub trait E57PointCloudSimpleExt<T: Seek + Read> {
    /// Converts an `e57::PointCloudReaderSimple` into a vector of `las::Point`.
    ///
    /// Implementations of this method should process the point cloud data and return a list of successfully
    /// converted points. The method may choose to handle invalid points by filtering them out or by some other mechanism.
    ///
    /// This is useful when you want to process each pointcloud separately.
    ///
    /// # Returns
    /// A `Vec<las::Point>` containing the points that were successfully converted.
    ///
    /// # Example
    /// ```
    /// use e57_to_las::e57::E57PointCloudSimpleExt;
    ///
    /// let mut e57_reader = e57::E57Reader::from_file("./examples/bunnyDouble.e57")
    ///                        .unwrap();
    ///
    /// let pointclouds = e57_reader.pointclouds();
    ///
    /// let first_pointcloud = pointclouds.iter().next().unwrap();
    ///
    /// let pointcloud_simple_reader = e57_reader.pointcloud_simple(first_pointcloud).unwrap();
    ///
    /// let las_points = pointcloud_simple_reader.x_to_las();
    ///
    /// assert!(!las_points.is_empty());
    /// ```
    fn x_to_las(self) -> Vec<las::Point>;
}

impl<'a, T: Read + Seek> E57PointCloudSimpleExt<T> for PointCloudReaderSimple<'a, T> {
    /// Converts an `e57::PointCloudReaderSimple` into a vector of `las::Point`.
    ///
    /// This implementation processes each `e57::Point` using the `x_to_las` method from the `E57PointExt` trait.
    /// If a point cannot be converted (e.g., due to invalid Cartesian coordinates), it is excluded from the result.
    fn x_to_las(self) -> Vec<las::Point> {
        self.into_iter()
            .filter_map(|p| p.ok().and_then(|p| p.x_to_las()))
            .collect()
    }
}
