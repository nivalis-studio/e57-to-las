//! A Rust lib to convert E57 point cloud files to LAS format.

#![forbid(unsafe_code)]
#![deny(
    clippy::panic,
    clippy::expect_used,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value
)]
#![warn(clippy::unwrap_used)]

mod error;

pub mod e57;
pub mod las;
pub use error::{Error, Result};
