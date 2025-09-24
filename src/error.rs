use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O operation failed")]
    Io(#[from] std::io::Error),

    #[error("E57 error")]
    E57(#[from] e57::Error),

    #[error("LAS error")]
    Las(#[from] las::Error),

    #[error("invalid LAS version: {0}")]
    InvalidLasVersion(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;
