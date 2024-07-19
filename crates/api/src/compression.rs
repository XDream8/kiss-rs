#[derive(Debug, Clone)]
pub enum CompressionType {
    BZ2,
    GZ,
    LZ4,
    XZ,
    ZSTD,
}

impl CompressionType {
    pub fn get_ext(&self) -> &'static str {
        match self {
            Self::BZ2 => "bz2",
            Self::GZ => "gz",
            Self::LZ4 => "lz4",
            Self::XZ => "xz",
            Self::ZSTD => "zstd",
        }
    }
}
