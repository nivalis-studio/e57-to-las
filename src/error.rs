//! Error types for E57 to LAS conversion operations.

use thiserror::Error;

/// Errors that can occur during E57 to LAS conversion.
///
/// This enum wraps errors from underlying libraries (E57, LAS, I/O) and adds
/// conversion-specific error variants.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O operation failed while reading E57 sources or writing LAS outputs.
    ///
    /// This can occur during file opening, reading, writing, or seeking operations.
    #[error("I/O operation failed")]
    Io(#[from] std::io::Error),

    /// Error from the E57 library while reading or parsing E57 sources.
    ///
    /// Common causes include:
    /// - Corrupted E57 structure
    /// - Unsupported E57 format features
    /// - Invalid XML structure in E57 header
    #[error("E57 error")]
    E57(#[from] e57::Error),

    /// Error from the LAS library while writing LAS outputs.
    ///
    /// Common causes include:
    /// - Invalid point data
    /// - Header configuration errors
    /// - Write failures during point serialization
    #[error("LAS error")]
    Las(#[from] las::Error),

    /// Invalid LAS version specified in conversion options.
    ///
    /// LAS versions must have major version 1 and minor version 0-4.
    /// See [`LasVersion`](crate::LasVersion) for valid versions.
    #[error("invalid LAS version: {0}")]
    InvalidLasVersion(String),

    /// Internal error indicating a bug or unexpected condition.
    ///
    /// These errors typically indicate:
    /// - Thread panics in parallel processing
    /// - Channel communication failures
    /// - Invariant violations
    ///
    /// If you encounter this error, please report it as a bug.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias using the crate's [`enum@Error`] type.
///
/// This is a convenience type for functions that return conversion results.
pub type Result<T> = std::result::Result<T, Error>;
