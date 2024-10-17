use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("package not found")]
    PackageNotFound,
    #[error("package is not installed")]
    PackageIsNotInstalled,
    #[error("version file not found")]
    VersionFileNotFound,
    #[error("sources file not found")]
    SourcesFileNotFound,
    #[error("root directory does not exists. be sure to provide a proper path")]
    RootDirNotExists,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("http client error: {0}")]
    HttpClient(String),
}

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        Error::HttpClient(e.to_string())
    }
}
