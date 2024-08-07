use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("package not found")]
    PackageNotFound,
    #[error("package is not installed")]
    PackageIsNotInstalled,
    #[error("Version file not found")]
    VersionFileNotFound,
    #[error("Sources file not found")]
    SourcesFileNotFound,
    #[error("IO error")]
    Io(#[from] std::io::Error),
}
