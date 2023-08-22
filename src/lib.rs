//! A Rust lib to convert E57 point cloud files to LAS format.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value
)]

mod convert_file;
mod convert_point;
mod convert_pointcloud;
mod get_las_writer;
mod stations;
mod utils;

pub use self::convert_file::convert_file;
pub use self::convert_pointcloud::convert_pointcloud;
pub use self::stations::StationPoint;
pub use self::stations::{create_station_file, create_station_point, get_sum_coordinates};
