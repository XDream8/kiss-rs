/// This file will mostly include functions for parsing sources file that almost every package should include
use std::path::{Path, PathBuf};

use crate::compression::CompressionType;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Source {
    pub source_type: SourceType,
    pub url: String,
    pub path_to_put_when_building: Option<PathBuf>,
    pub source_file_path: Option<PathBuf>,
    pub extract_archive: bool,
}

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum SourceType {
    Git,
    Http,
    Local,
}

pub fn parse_source_line(
    line: &str,
    package_name: &String,
    package_path: Option<&Path>,
    source_cache_dir: Option<&PathBuf>,
) -> Option<Source> {
    let trimmed = line.trim();

    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    // Split the line into parts
    let mut parts = trimmed.split_whitespace();
    let url = parts.next()?;
    let path_to_put_when_building = parts.next().map(PathBuf::from);

    // put together information and determine where source file/dir/archive should be in the cache directory
    let mut source_file_path = if let Some(source_cache_dir) = source_cache_dir {
        let file_name: &str = extract_repo_or_file_name(url).unwrap_or_else(|| {
            println!("Unable to extract repository name from Git URL");
            std::process::exit(1);
        });

        let mut temp_source_file_path = source_cache_dir.join(package_name);

        if let Some(opt_path) = &path_to_put_when_building {
            temp_source_file_path.push(opt_path)
        }
        temp_source_file_path.push(file_name);
        Some(temp_source_file_path)
    } else {
        None
    };

    let source_type = match url {
        u if u.starts_with("git+") => SourceType::Git,
        u if u.starts_with("http://") || u.starts_with("https://") => source_file_path
            .as_deref()
            .filter(|path| path.exists())
            .map_or(SourceType::Http, |_| SourceType::Local),
        _ => {
            if let Some(package_path_in_repo) = package_path {
                source_file_path = Some(package_path_in_repo.join(url));
            }
            SourceType::Local
        }
    };

    let extract_archive: bool = !url.ends_with("?no-extract");

    Some(Source {
        source_type,
        url: url.to_string(),
        path_to_put_when_building,
        source_file_path,
        extract_archive,
    })
}

pub fn extract_repo_or_file_name(url: &str) -> Option<&str> {
    let path_segments: Vec<&str> = url.trim_end_matches('/').split('/').collect();

    if let Some(last_segment) = path_segments.last() {
        // Check if the URL ends with ".git" and remove it
        if last_segment.ends_with(".git") {
            return Some(last_segment.trim_end_matches(".git"));
        }
        // Check if the repo_name is not empty
        if !last_segment.is_empty() {
            return Some(last_segment);
        }
    }

    None
}

pub fn pkg_is_binary_available(
    bin_dir: &Path,
    package_name: &str,
    package_version: &String,
    compression_type: &CompressionType,
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

// TESTS
#[cfg(test)]
pub mod tests {
    use std::path::PathBuf;

    use super::{parse_source_line, Source, SourceType};

    #[test]
    fn parse_source_line_empty_line() {
        let line = "";
        let res = None;
        assert_eq!(
            parse_source_line(line, &String::from("test"), None, None),
            res
        )
    }

    #[test]
    fn parse_source_line_local_file() {
        let line = "patches/fix.patch";
        let res = Some(Source {
            source_type: SourceType::Local,
            url: String::from("patches/fix.patch"),
            path_to_put_when_building: None,
            source_file_path: None,
            extract_archive: true,
        });
        assert_eq!(
            parse_source_line(line, &String::from("test"), None, None),
            res
        )
    }
    #[test]
    fn parse_source_line_local_file_with_path() {
        let line = "patches/fix.patch fix.patch";
        let res = Some(Source {
            source_type: SourceType::Local,
            url: String::from("patches/fix.patch"),
            path_to_put_when_building: Some(PathBuf::from("fix.patch")),
            source_file_path: None,
            extract_archive: true,
        });
        assert_eq!(
            parse_source_line(line, &String::from("test"), None, None),
            res
        )
    }

    #[test]
    fn parse_source_line_remote_file() {
        let line = "https://codeberg.org/XDream8/kiss-rs/archive/v1.0.tar.gz";
        let res = Some(Source {
            source_type: SourceType::Http,
            url: String::from("https://codeberg.org/XDream8/kiss-rs/archive/v1.0.tar.gz"),
            path_to_put_when_building: None,
            source_file_path: None,
            extract_archive: true,
        });
        assert_eq!(
            parse_source_line(line, &String::from("test"), None, None),
            res
        )
    }
    #[test]
    fn parse_source_line_remote_file_with_path() {
        let line = "https://codeberg.org/XDream8/kiss-rs/archive/v1.0.tar.gz kiss-rs-latest";
        let res = Some(Source {
            source_type: SourceType::Http,
            url: String::from("https://codeberg.org/XDream8/kiss-rs/archive/v1.0.tar.gz"),
            path_to_put_when_building: Some(PathBuf::from("kiss-rs-latest")),
            source_file_path: None,
            extract_archive: true,
        });
        assert_eq!(
            parse_source_line(line, &String::from("test"), None, None),
            res
        )
    }

    #[test]
    fn parse_source_line_git() {
        let line = "git+https://codeberg.org/XDream8/kiss-rs";
        let res = Some(Source {
            source_type: SourceType::Git,
            url: String::from("git+https://codeberg.org/XDream8/kiss-rs"),
            path_to_put_when_building: None,
            source_file_path: None,
            extract_archive: true,
        });
        assert_eq!(
            parse_source_line(line, &String::from("test"), None, None),
            res
        )
    }
    #[test]
    fn parse_source_line_git_with_path() {
        let line = "git+https://codeberg.org/XDream8/kiss-rs kiss-rs-latest";
        let res = Some(Source {
            source_type: SourceType::Git,
            url: String::from("git+https://codeberg.org/XDream8/kiss-rs"),
            path_to_put_when_building: Some(PathBuf::from("kiss-rs-latest")),
            source_file_path: None,
            extract_archive: true,
        });
        assert_eq!(
            parse_source_line(line, &String::from("test"), None, None),
            res
        )
    }
}
