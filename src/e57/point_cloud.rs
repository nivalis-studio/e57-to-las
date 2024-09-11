use std::io::{Read, Seek};

use e57::PointCloudReaderSimple;

use super::E57PointExt;

pub trait E57PointCloudSimpleExt<T: Seek + Read> {
    fn to_las_points(self) -> Vec<las::Point>;
}

impl<'a, T: Read + Seek> E57PointCloudSimpleExt<T> for PointCloudReaderSimple<'a, T> {
    fn to_las_points(self) -> Vec<las::Point> {
        self.into_iter()
            .filter_map(|p| p.ok().and_then(|p| p.to_las_point()))
            .collect()
    }
}
