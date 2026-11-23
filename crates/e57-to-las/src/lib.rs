//! Convert E57 point clouds to LAS and LAZ.
//!
//! - **Streaming**: points go from the E57 reader to the LAS writer in batches;
//!   memory use does not grow with the file.
//! - **Parallel**: point clouds are decoded on several threads (feature `parallel`).
//! - **Flexible outputs**: one merged file, one file per scan, custom groups,
//!   files named from a template, in-memory buffers or any `Write + Seek`.
//! - **Safe output**: files appear only once complete; a failed or cancelled
//!   conversion removes what it started.
//! - **Observable**: progress events, warnings and cancellation.
//!
//! # Quick start
//!
//! ```no_run
//! let summary = e57_to_las::convert("scan.e57", "scan.las")?;
//! println!("wrote {} points", summary.points_written());
//! # Ok::<(), e57_to_las::Error>(())
//! ```
//!
//! # Configuration
//!
//! [`Converter`] holds every option:
//!
//! ```no_run
//! use e57_to_las::{Converter, Grouping, LasVersion, output::FileOutput};
//!
//! let summary = Converter::new()
//!     .las_version(LasVersion::V1_4)
//!     .grouping(Grouping::PerCloud)
//!     .stable_order(true)
//!     .observer(|event| eprintln!("{event:?}"))
//!     .convert("scan.e57", FileOutput::new("out/{stem}_{index}.las"))?;
//!
//! for output in &summary.outputs {
//!     println!("{:?}: {} points", output.location, output.points_written());
//! }
//! # Ok::<(), e57_to_las::Error>(())
//! ```
//!
//! # What is converted
//!
//! Each point keeps its coordinates (with the scan pose applied), color,
//! intensity, GPS time, return number and count, and the index of its point
//! cloud as LAS point source ID. Surface normals and scan grid row/column can
//! be added as LAS extra bytes (see [`Attributes`]). The E57 coordinate
//! reference system is copied when it is WKT and the output is LAS 1.4
//! (see [`Crs`]). Points without valid, finite coordinates are skipped and
//! counted.
//!
//! Points are written with legacy LAS point formats 0 to 3, which every LAS
//! reader supports, at 1 mm precision by default (see [`Precision`]).
//!
//! # Features
//!
//! - `parallel` (default): decode point clouds on a thread pool.
//! - `crc32c` (default): hardware-accelerated E57 checksums.
//! - `laz`: write LAZ. `laz-parallel` also compresses on several threads.
//! - `serde`: `Serialize`/`Deserialize` for the public data types.

mod cloud;
mod converter;
mod decode;
mod engine;
mod error;
mod header;
mod input;
mod layout;
mod observe;
pub mod output;
mod point;
mod precision;
mod scan;
mod summary;
mod version;

pub use cloud::{Bounds, CloudInfo, Pose};
pub use converter::{Converter, Grouping};
pub use error::{Error, Result, Warning};
pub use header::{Crs, HeaderOptions, Vlr};
pub use input::Input;
pub use observe::{CancelToken, Event};
pub use output::GroupKey;
pub use point::{Attributes, Point, Rgb};
pub use precision::Precision;
pub use scan::Scan;
pub use summary::{CloudSummary, OutputSummary, Summary};
pub use version::{LasVersion, ParseLasVersionError};

/// Converts `input` to one LAS file at `output` with the default options.
///
/// Every point cloud is merged into the one file. A `.laz` extension writes
/// LAZ (feature `laz`). Use [`Converter`] for anything else.
pub fn convert(input: impl Into<Input>, output: impl AsRef<std::path::Path>) -> Result<Summary> {
    Converter::new().convert(input, output::FileOutput::new(output.as_ref()))
}
