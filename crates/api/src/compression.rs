pub enum CompressionType {
    BZ2,
    GZ,
    LZ4,
    XZ2,
    ZSTD,
}

impl CompressionType {
    pub fn get_ext(self) -> &'static str {
        match self {
            Self::BZ2 => "bz2",
            Self::GZ => "gz",
            Self::LZ4 => "lz4",
            Self::XZ2 => "xz2",
            Self::ZSTD => "zstd",
        }
    }
}
