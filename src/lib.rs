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
mod convert_pointcloud;
mod error;
mod get_las_writer;
mod las_version;
mod point;
mod utils;

pub mod stations;

pub use self::convert_file::convert_file;
pub use self::convert_pointcloud::convert_pointcloud;
pub use error::{Error, Result};
pub use las_version::LasVersion;
pub use point::E57PointExt;
