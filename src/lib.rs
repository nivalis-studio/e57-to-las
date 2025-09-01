//! A Rust lib to convert E57 point cloud files to LAS format.

#![forbid(unsafe_code)]
#![deny(
    clippy::panic,
    clippy::expect_used,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value
)]
#![warn(clippy::unwrap_used)]

mod convert_file;
mod convert_point;
mod convert_pointcloud;
#[cfg(test)]
mod convert_pointcloud_old;
mod error;
mod get_las_writer;
mod las_version;
mod spatial_point;
mod stations;
mod utils;

pub use self::convert_file::convert_file;
pub use self::convert_point::convert_point;
pub use self::convert_pointcloud::convert_pointcloud;
pub use error::{Error, Result};
pub use las_version::LasVersion;
