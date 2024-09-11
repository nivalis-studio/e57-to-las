use super::E57PointCloudSimpleExt;
use e57::E57Reader;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    io::{Read, Seek},
    sync::Mutex,
};

pub trait E57ReaderExt {
    fn x_to_las(&mut self) -> Vec<las::Point>;
}

impl<T: Read + Seek + Send + Sync> E57ReaderExt for E57Reader<T> {
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
