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

impl From<&str> for CompressionType {
    fn from(s: &str) -> CompressionType {
        match s {
            "bz2" => CompressionType::BZ2,
            "gz" => CompressionType::GZ,
            "lz4" => CompressionType::LZ4,
            "xz" => CompressionType::XZ,
            "zstd" => CompressionType::ZSTD,
            _ => panic!("Unknown compression type: {}", s),
        }
    }
}
