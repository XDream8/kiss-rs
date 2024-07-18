use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::compression::CompressionType;
use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Source {
    pub source_type: SourceType,
    pub url: String,
    pub path: PathBuf,
    pub path_to_put_when_building: Option<PathBuf>,
}

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum SourceType {
    Git,
    Http,
    Local,
}

pub fn parse_source_line(line: &str) -> Option<Source> {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return None;
    }

    // Split the line into parts
    let mut parts = trimmed.split_whitespace();
    let url = parts.next()?;
    let path_to_put_when_building = parts.next().map(PathBuf::from);

    let source_type = if url.starts_with("https://") || url.starts_with("http://") {
        SourceType::Http
    } else if url.starts_with("git://") || url.ends_with(".git") {
        SourceType::Git
    } else {
        SourceType::Local
    };

    Some(Source {
        source_type,
        url: url.to_string(),
        path: PathBuf::from(url),
        path_to_put_when_building,
    })
}

pub fn parse_source_file<P: AsRef<Path>>(file_path: P) -> Result<Vec<Source>, Error> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut sources = Vec::new();

    for line in reader.lines() {
        if let Some(source) = parse_source_line(&line?) {
            sources.push(source);
        }
    }

    Ok(sources)
}

pub fn pkg_is_binary_available(
    bin_dir: &Path,
    package_name: &str,
    package_version: &String,
    compression_type: CompressionType,
) -> Option<PathBuf> {
    let file = bin_dir.join(format!("{}@{}.tar", package_name, package_version));
    let ext = compression_type.get_ext();
    let file_with_ext: PathBuf = file.with_extension(format!("tar.{ext}"));

    if file_with_ext.exists() {
        return Some(file_with_ext);
    }

    // Try different extensions -- instead of iterating through the binary directory just check the files with exactly these extensions
    let alternative_extensions = ["tar.gz", "tar.bz2", "tar.xz", "tar.lz4"];
    for alt_ext in &alternative_extensions {
        let alt_file = file.with_extension(alt_ext);
        if alt_file.exists() {
            return Some(alt_file);
        }
    }

    None
}
