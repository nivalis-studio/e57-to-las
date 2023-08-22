#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value
)]

pub mod colors;
pub mod p_converter;
pub mod pc_converter;
pub mod stations;
pub mod utils;
