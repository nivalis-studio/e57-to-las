#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid LAS version {0}")]
    InvalidLasVersion(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub type Result<T> = core::result::Result<T, Error>;
