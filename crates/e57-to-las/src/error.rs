use std::path::PathBuf;

use crate::{GroupKey, LasVersion};

/// An error from another library (the E57 reader, the LAS writer or I/O),
/// kept as an opaque [`source`](std::error::Error::source).
pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Why a conversion failed.
///
/// Errors coming from the E57 and LAS libraries are reachable through
/// [`std::error::Error::source`] but their types are not part of this API.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The input could not be opened.
    #[error("failed to open {}", .path.as_ref().map_or_else(|| "the input".to_owned(), |p| p.display().to_string()))]
    Open {
        /// The input path, when the input is a file.
        path: Option<PathBuf>,
        /// The I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The input is not a readable E57 file.
    #[error("the input is not a valid E57 file")]
    InvalidE57(#[source] BoxError),

    /// Reading the points of a point cloud failed.
    #[error("failed to read point cloud {cloud}")]
    Read {
        /// Index of the point cloud in the E57 file.
        cloud: usize,
        /// The underlying error.
        #[source]
        source: BoxError,
    },

    /// Creating, writing or finishing an output failed.
    #[error("failed to write output {output}")]
    Write {
        /// The output: its path when it has one, otherwise its group key.
        output: String,
        /// The underlying error.
        #[source]
        source: BoxError,
    },

    /// The LAS version has no point format with colors (LAS 1.0 and 1.1).
    #[error(
        "LAS {version} cannot store point colors, but the input has colors: use LAS 1.2 or newer, or disable colors"
    )]
    ColorUnsupported {
        /// The requested version.
        version: LasVersion,
    },

    /// A WKT coordinate reference system was requested for a LAS version older than 1.4.
    #[error("a WKT coordinate reference system needs LAS 1.4, not LAS {version}")]
    CrsUnsupported {
        /// The requested version.
        version: LasVersion,
    },

    /// LAZ output was requested but the crate was built without the `laz` feature.
    #[error("LAZ output needs the `laz` feature of e57-to-las")]
    LazNotEnabled,

    /// Two outputs resolve to the same location, so one would overwrite the other.
    #[error(
        "outputs {first} and {second} would both be written to {}: add a placeholder such as {{index}} to the output path",
        .location.display()
    )]
    OutputCollision {
        /// The shared location.
        location: PathBuf,
        /// Key of the first output.
        first: GroupKey,
        /// Key of the second output.
        second: GroupKey,
    },

    /// A point does not fit the scale and offset given with [`Precision::Fixed`](crate::Precision::Fixed).
    #[error(
        "point ({x}, {y}, {z}) of point cloud {cloud} does not fit the fixed LAS scale and offset"
    )]
    PointOutOfRange {
        /// Index of the point cloud.
        cloud: usize,
        /// X coordinate.
        x: f64,
        /// Y coordinate.
        y: f64,
        /// Z coordinate.
        z: f64,
    },

    /// An option has an invalid value.
    #[error("invalid option: {0}")]
    InvalidOption(String),

    /// The conversion was cancelled through a [`CancelToken`](crate::CancelToken).
    #[error("the conversion was cancelled")]
    Cancelled,
}

/// Shorthand for `Result<T, e57_to_las::Error>`.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Something worth knowing about a conversion that still succeeded.
///
/// Warnings are listed in [`Summary::warnings`](crate::Summary::warnings) and
/// sent to the observer as [`Event::Warning`](crate::Event::Warning). The
/// library never prints them.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Warning {
    /// A GUID in the E57 file is not a UUID, so the LAS GUID was left empty.
    #[error("GUID {guid:?} is not a UUID: the LAS GUID of output {output} was left empty")]
    InvalidGuid {
        /// The output whose header lacks the GUID.
        output: GroupKey,
        /// The GUID found in the E57 file.
        guid: String,
    },

    /// The points span more than the requested scale allows (about 2,147 km at
    /// 1 mm), so a coarser scale was used.
    #[error(
        "output {output}: points span too far for a scale of {requested}, wrote with scale {used:?}"
    )]
    CoarseScale {
        /// The output.
        output: GroupKey,
        /// The requested scale.
        requested: f64,
        /// The scale used per axis.
        used: [f64; 3],
    },

    /// A point cloud has timestamps but no acquisition start time, so its
    /// times are written as seconds since the start of the scan.
    #[error(
        "point cloud {cloud} has no acquisition start: its GPS times are seconds since the scan start"
    )]
    RelativeTime {
        /// Index of the point cloud.
        cloud: usize,
    },

    /// The coordinate reference system of the E57 file was not written.
    #[error("the coordinate reference system was not written: {reason}")]
    CrsNotWritten {
        /// Why.
        reason: String,
    },

    /// Some points have more than 7 returns, the most legacy LAS point formats store.
    #[error("point cloud {cloud}: {count} points had return numbers above 7, written as 7")]
    ReturnsClamped {
        /// Index of the point cloud.
        cloud: usize,
        /// How many points were clamped.
        count: u64,
    },

    /// A point cloud index above 65,535 does not fit the LAS point source ID.
    #[error("point cloud {cloud}: its index does not fit the LAS point source ID, wrote 65535")]
    PointSourceIdClamped {
        /// Index of the point cloud.
        cloud: usize,
    },
}
