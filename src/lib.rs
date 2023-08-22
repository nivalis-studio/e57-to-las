//! A Rust lib to convert E57 point cloud files to LAS format.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value
)]

mod colors;
mod convert_point;
mod convert_pointcloud;
mod stations;
mod utils;

pub use self::colors::{get_colors_limit, get_intensity_limits, get_las_colors, get_las_intensity};
pub use self::colors::{ParsedColorLimits, ParsedIntensityLimits};
pub use self::convert_point::convert_point;
pub use self::convert_pointcloud::convert_pointcloud;
pub use self::stations::{create_station_point, get_sum_coordinate, StationPoint};
