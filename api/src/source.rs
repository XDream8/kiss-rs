/// This file will mostly include functions for parsing sources file that almost every package should include
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use once_cell::sync::Lazy;
use ureq::{Agent, AgentBuilder, Response};

use crate::{common_funcs::tmp_file, compression::CompressionType, error::Error};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Source {
    pub source_type: SourceType,
    pub url: String,
    pub path_to_put_when_building: Option<PathBuf>,
    pub source_file_path: Option<PathBuf>,
    pub source_file_name: String,
    pub extract_archive: bool,
}

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum SourceType {
    Git,
    Http,
    Local,
}

// reusable lazy initialized HTTP CLIENT
pub static HTTP_CLIENT: Lazy<Agent> = Lazy::new(|| {
    AgentBuilder::new()
        .timeout_read(Duration::from_secs(10))
        .timeout_write(Duration::from_secs(10))
        .build()
});

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

    let source_file_name: String = extract_repo_or_file_name(url).unwrap_or_else(|| {
        println!("Unable to extract repository name from Git URL");
        std::process::exit(1);
    });

    // put together information and determine where source file/dir/archive should be in the cache directory
    let mut source_file_path = if let Some(source_cache_dir) = source_cache_dir {
        let mut temp_source_file_path = source_cache_dir.join(package_name);

        if let Some(opt_path) = &path_to_put_when_building {
            temp_source_file_path.push(opt_path)
        }
        temp_source_file_path.push(&source_file_name);
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
        source_file_name,
        extract_archive,
    })
}

pub fn extract_repo_or_file_name(url: &str) -> Option<String> {
    let path_segments: Vec<&str> = url.trim_end_matches('/').split('/').collect();

    if let Some(last_segment) = path_segments.last() {
        // Check if the URL ends with ".git" and remove it
        if last_segment.ends_with(".git") {
            return Some(last_segment.trim_end_matches(".git").to_owned());
        }
        // Check if the repo_name is not empty
        if !last_segment.is_empty() {
            return Some(last_segment.to_string());
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

pub fn pkg_download_source(
    package_name: &String,
    sources: &Vec<Source>,
    tmp_dir: &Path,
) -> Result<(), Error> {
    for source in sources {
        if source.source_type == SourceType::Http {
            pkg_source_url(package_name, source, tmp_dir)?
        }
    }

    Ok(())
}

// Function to download files
pub fn pkg_source_url(package_name: &String, source: &Source, tmp_dir: &Path) -> Result<(), Error> {
    println!("{}: Downloading: {}", package_name, source.url);

    let response: Response = HTTP_CLIENT.get(&source.url).call()?;

    let total_size: u64 = response
        .header("Content-Length")
        .and_then(|length| length.parse::<u64>().ok())
        .unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut buffer: [u8; 8192] = [0; 8192];

    // get file_name from download_dest variable
    //let file_name: String = download_dest
    //    .to_string_lossy()
    //    .split('/')
    //    .last()
    //    .unwrap()
    //    .to_owned();

    // tmp file
    dbg!(&tmp_dir);
    let (mut tmp_file, tmp_file_path) =
        tmp_file(tmp_dir, source.source_file_name.as_str(), "download")?;

    let mut response_reader = response.into_reader();

    while let Ok(bytes_read) = response_reader.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }

        downloaded += bytes_read as u64;

        print_progress(downloaded, total_size);

        tmp_file.write_all(&buffer[..bytes_read])?;
    }

    println!("\rDownloading {}: 100% (Completed)", source.url);

    let dest_path = if let Some(dest) = &source.source_file_path {
        dest
    } else {
        todo!()
    };
    // move tmp_file
    std::fs::rename(tmp_file_path, dest_path)?;

    Ok(())
}

pub fn print_progress(progress: u64, total_size: u64) {
    let formatted_progress: String = convert_bytes(progress);
    if total_size == 0 {
        print!("\rDownloading... ({}/Unknown)", formatted_progress);
    } else {
        let percent: f64 = (progress as f64 / total_size as f64) * 100.0;
        let formatted_total_size: String = convert_bytes(total_size);
        print!(
            "\rDownloading... {:.2}% ({}/{})",
            percent, formatted_progress, formatted_total_size
        );
    }
    std::io::stdout().flush().unwrap();
}

pub fn convert_bytes(bytes: u64) -> String {
    const UNIT: u64 = 1024;
    if bytes < UNIT {
        return format!("{} B", bytes);
    }
    let exp: u32 = (bytes as f64).log(UNIT as f64) as u32;
    let pre = "KMGTPE".chars().nth(exp as usize - 1).unwrap();
    let value: f64 = bytes as f64 / f64::powi(UNIT as f64, exp as i32);
    format!("{:.1} {}B", value, pre)
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
            source_file_name: String::from("fix.patch"),
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
            source_file_name: String::from("fix.patch"),
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
            source_file_name: String::from("v1.0.tar.gz"),
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
            source_file_name: String::from("v1.0.tar.gz"),
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
            source_file_name: String::from("kiss-rs"),
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
            source_file_name: String::from("kiss-rs"),
            extract_archive: true,
        });
        assert_eq!(
            parse_source_line(line, &String::from("test"), None, None),
            res
        )
    }
}
