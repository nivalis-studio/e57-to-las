/// The error type for operations in the `e57_to_las` crate.
///
/// This enum represents the different errors that can occur while converting E57 files to LAS format
/// or interacting with LAS data. It includes custom errors as well as a general fallback for unexpected issues.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Represents an invalid LAS version.
    #[error("Invalid LAS version {0}")]
    InvalidLasVersion(String),

    /// A wrapper for unexpected errors.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

/// A specialized `Result` type for operations in the `e57_to_las` crate.
///
/// This type is a shorthand for results that return either the successful type `T` or an [`Error`].
pub type Result<T> = core::result::Result<T, Error>;
