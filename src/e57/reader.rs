use super::E57PointCloudSimpleExt;
use crate::{Error, Result};
use anyhow::{anyhow, Context};
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
    /// with large point clouds. Errors encountered during the conversion process, such as issues with point cloud
    /// retrieval or parallel processing, are propagated and returned.
    ///
    /// # Returns
    /// A [`Result`]`<Vec<las::Point>>` containing either:
    /// - A `Vec<las::Point>` with the combined LAS points from all point clouds in the E57 file if successful.
    /// - An [`Error`] if any issues occur during the conversion or processing, such as failing to acquire a mutex lock,
    ///   failing to retrieve a point cloud, or other issues.
    ///
    /// # Example
    /// ```
    /// use e57_to_las::e57::E57ReaderExt;
    ///
    /// let mut e57_reader = e57::E57Reader::from_file("./examples/bunnyDouble.e57")
    ///                      .unwrap();
    ///
    /// let las_points = e57_reader.x_to_las();
    ///
    /// match las_points {
    ///     Ok(points) => {
    ///         assert!(!points.is_empty());
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Error converting E57 to LAS: {}", e);
    ///     }
    /// }
    /// ```
    fn x_to_las(&mut self) -> Result<Vec<las::Point>>;
}

impl<T: Read + Seek + Send + Sync> E57ReaderExt for E57Reader<T> {
    /// Converts all point clouds within the `e57::E57Reader` to `las::Point`.
    ///
    /// This implementation processes each point cloud in parallel using Rayon, converting them to LAS format by
    /// leveraging the `x_to_las` method from the `E57PointCloudSimpleExt` trait. It locks the `E57Reader` instance
    /// to ensure thread safety while reading point clouds from the file.
    ///
    /// Errors encountered during the processing, such as failing to acquire a mutex lock or failing to retrieve
    /// point clouds, are properly handled and propagated. The result will be a `Result` type indicating success or failure.
    ///
    /// # Example
    /// See the trait method documentation for a usage example.
    fn x_to_las(&mut self) -> Result<Vec<las::Point>> {
        let point_clouds = self.pointclouds();
        let reader_mutex = Mutex::new(self);

        // Process each point cloud in parallel and collect results
        let results: Result<Vec<las::Point>> = point_clouds
            .par_iter()
            .map(|pc| {
                let mut reader = reader_mutex
                    .lock()
                    .map_err(|_| Error::UnexpectedError(anyhow!("Failed to acquire mutex lock")))?;

                let point_cloud_simple = reader
                    .pointcloud_simple(pc)
                    .context("Failed to get point cloud reader")?;

                Ok(point_cloud_simple.x_to_las())
            })
            .collect::<Result<Vec<_>>>()
            .map(|results| results.into_iter().flatten().collect());

        results
    }
}
