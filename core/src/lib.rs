//! Convert E57 point cloud sources to LAS/LAZ format.
//!
//! This crate provides efficient conversion between E57 (ASTM E2807 standard XML-based
//! point cloud format) and LAS (ASPRS LIDAR data exchange format). It supports both
//! sequential and parallel processing, multiple point clouds per source, and flexible
//! I/O abstractions.
//!
//! # Acknowledgements
//!
//! This crate builds on the [`e57`] and [`las`] crates for format handling:
//! - **E57**: See the [`e57` crate documentation](https://docs.rs/e57/)
//! - **LAS**: See the [`las` crate documentation](https://docs.rs/las/)
//!
//! # Usage
//!
//! ## Basic Single-Source Conversion
//!
//! Convert an E57 source containing multiple point clouds into a single merged LAS output:
//!
//! ```no_run
//! use e57_to_las::{convert, ConvertOptions};
//!
//! convert("scan.e57", "output.las", &ConvertOptions::default())?;
//! ```
//!
//! ## Split Multiple Point Clouds
//!
//! Convert each point cloud in an E57 source to separate LAS outputs:
//!
//! ```no_run
//! use e57_to_las::{convert_split, ConvertOptions};
//!
//! // Creates output_0.las, output_1.las, etc.
//! let writers = convert_split("scan.e57", &"output.las", &ConvertOptions::default())?;
//! println!("Created {} LAS outputs", writers.len());
//! ```
//!
//! ## Parallel Processing
//!
//! Use parallel conversion for faster processing on multi-core systems:
//!
//! ```no_run
//! use e57_to_las::{parallel, ConvertOptions};
//!
//! // Note: Use &"input.e57" (reference) for ReaderFactory
//! parallel::convert(&"large_scan.e57", "output.las", &ConvertOptions::default())?;
//! ```
//!
//! ## Custom Configuration
//!
//! Configure scale, LAS version, and receive progress events:
//!
//! ```no_run
//! use e57_to_las::{convert, ConvertOptions, Scale, LasVersion, Event};
//! use std::sync::Arc;
//!
//! let options = ConvertOptions {
//!     scale: Scale::MilliMeter,
//!     las_version: LasVersion::new(1, 4)?,
//!     on_event: Some(Arc::new(|event| {
//!         match event {
//!             Event::PointCloudStarted { idx, name, points_count, .. } => {
//!                 println!("Processing cloud {}: {:?} ({} points)",
//!                     idx, name, points_count);
//!             },
//!             Event::PointCloudEnded { idx } => {
//!                 println!("Completed cloud {}", idx);
//!             },
//!             _ => {}
//!         }
//!     })),
//!     ..Default::default()
//! };
//!
//! convert("scan.e57", "output.las", &options)?;
//! ```
//!
//! see [`ConvertOptions`] for more details on customisation options
//!
//! # Coordinate System Handling
//!
//! The E57 crate automatically applies any transformations (rotation and translation)
//! defined in the E57 source before points are converted to LAS format. The LAS coordinate
//! system is configured with scale and offset transforms computed from the global bounds
//! of all point clouds being converted.
//!
//! # Features
//!
//! - `parallel` (default): Enable multi-threaded conversion for improved performance

mod convert;
mod error;
mod ext;
pub mod io;

pub use convert::{
    ConvertOptions, Event, EventCallback, HeaderHook, LasVersion, Scale, convert, convert_split,
    parallel,
};
pub use error::{Error, Result};
