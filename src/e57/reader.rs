use super::E57PointCloudSimpleExt;
use e57::E57Reader;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    io::{Read, Seek},
    sync::Mutex,
};

/// An extension trait for the `e57::E57Reader` type, adding functionality to convert all point clouds in an E57 file to LAS format.
pub trait E57ReaderExt {
    /// Converts all point clouds within an `e57::E57Reader` to `las::Point` objects.
    ///
    /// This method iterates over all point clouds in the E57 file, converting each point cloud to LAS format in parallel.
    /// It uses the `x_to_las` method from the `E57PointCloudSimpleExt` trait to convert individual point clouds.
    ///
    /// The implementation leverages parallel processing via Rayon to speed up the conversion process when working
    /// with large point clouds.
    ///
    /// # Returns
    /// A `Vec<las::Point>` containing the combined LAS points from all point clouds in the E57 file.
    ///
    /// # Example
    /// ```
    /// use e57_to_las::e57::E57ReaderExt;
    ///
    /// let mut e57_reader = e57::E57Reader::from_file("./examples/bunnyDouble.e57")
    ///                      .unwrap();
    ///
    /// let las_points: Vec<las::Point> = e57_reader.x_to_las();
    ///
    /// assert!(!las_points.is_empty());
    /// ```
    fn x_to_las(&mut self) -> Vec<las::Point>;
}

impl<T: Read + Seek + Send + Sync> E57ReaderExt for E57Reader<T> {
    /// Converts all point clouds within the `e57::E57Reader` to `las::Point`.
    ///
    /// This implementation processes each point cloud in parallel using Rayon, converting them to LAS format by
    /// leveraging the `x_to_las` method from the `E57PointCloudSimpleExt` trait. It locks the `E57Reader` instance
    /// to ensure thread safety while reading point clouds from the file.
    ///
    /// # Example
    /// See the trait method documentation for a usage example.
    fn x_to_las(&mut self) -> Vec<las::Point> {
        let point_clouds = self.pointclouds();
        let reader_mutex = Mutex::new(self);

        point_clouds
            .par_iter()
            .flat_map(|pc| {
                let mut reader = reader_mutex.lock().unwrap();
                let point_cloud_simple = reader.pointcloud_simple(pc).unwrap();

                point_cloud_simple.x_to_las()
            })
            .collect()
    }
}
