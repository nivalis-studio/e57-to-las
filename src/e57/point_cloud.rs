use super::E57PointExt;
use e57::PointCloudReaderSimple;
use std::io::{Read, Seek};

pub trait E57PointCloudSimpleExt<T: Seek + Read> {
    fn x_to_las(self) -> Vec<las::Point>;
}

impl<'a, T: Read + Seek> E57PointCloudSimpleExt<T> for PointCloudReaderSimple<'a, T> {
    fn x_to_las(self) -> Vec<las::Point> {
        self.into_iter()
            .filter_map(|p| p.ok().and_then(|p| p.x_to_las()))
            .collect()
    }
}
