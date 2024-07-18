use std::error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to open file")]
    FileOpenError,
    #[error("package not found")]
    PackageNotFound,
    #[error("package is not installed")]
    PackageIsNotInstalled,
    #[error("Version file not found")]
    VersionFileNotFound,
    #[error("IO error")]
    Io(#[from] std::io::Error),
}
