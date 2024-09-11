//! A rust crate for converting E57 point cloud files to the LAS format.
//!
//! This crate provides extension traits for elements of both the [`e57`](https://docs.rs/e57/) and [`las`](https://docs.rs/las/) crates,
//! enabling easier conversion from E57 to LAS.

#![forbid(unsafe_code)]
#![deny(
    clippy::panic,
    clippy::expect_used,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value
)]
#![warn(clippy::unwrap_used)]

mod error;

/// extension traits for [`e57`](https://docs.rs/e57/) crate elements
pub mod e57;
/// extension traits for [`las`](https://docs.rs/las/) crate elements
pub mod las;
pub use error::{Error, Result};
