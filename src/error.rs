use std::path::PathBuf;

use crate::LasVersion;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid LAS version {0}")]
    InvalidLasVersion(String),

    /// LAS 1.0 and 1.1 have no point format that can store color.
    #[error(
        "LAS {version} cannot store point colors, but the input contains colored point clouds. Use LAS version 1.2 or newer."
    )]
    ColorUnsupported { version: LasVersion },

    /// The input path could not be opened. The source is the underlying I/O error.
    #[error("failed to open {path}")]
    Open {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The file opened, but it is not an ASTM E57 point cloud.
    #[error("{path} is not an ASTM E57 file")]
    InvalidFormat { path: PathBuf },

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Error {
    /// Adds context to an [`Error::UnexpectedError`] and leaves typed errors intact.
    pub(crate) fn context(self, message: impl std::fmt::Display + Send + Sync + 'static) -> Self {
        match self {
            Error::UnexpectedError(err) => Error::UnexpectedError(err.context(message)),
            other => other,
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
