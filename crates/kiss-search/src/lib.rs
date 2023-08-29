// global variables
use shared_lib::{read_a_dir_and_sort, read_a_files_lines};

use shared_lib::globals::Config;

// logging
use shared_lib::die;
use shared_lib::signal::pkg_clean;

// threading
#[cfg(feature = "threading")]
use rayon::iter::ParallelIterator;
use shared_lib::iter;

use std::fs;
use std::path::{Path, PathBuf};

// find and return only one package and version
// this is primarily used by functions!
// returns ’version_number-release’
pub fn pkg_find_version(
    config: &Config,
    name: &str,
    search_path: Option<&String>,
) -> Option<String> {
    let kiss_path: Vec<&String> = if let Some(search_path) = search_path {
        vec![search_path]
    } else {
        Vec::from_iter(config.kiss_path.iter())
    };

    // Use Rayon to parallelize the iteration through kiss_path directories
    let found_packages: Vec<PathBuf> = iter!(kiss_path)
        .flat_map(|path| read_a_dir_and_sort(path.as_str(), false, &[]))
        .filter(|package| {
            let package_name = match package.file_name() {
                Some(file_name) => file_name.to_string_lossy(),
                _ => std::borrow::Cow::Borrowed(""),
            };

            name == package_name
        })
        .collect();

    if !found_packages.is_empty() {
        extract_package_version(&found_packages[0])
    } else {
        None
    }
}

pub fn pkg_find_path(config: &Config, name: &str, search_path: Option<&String>) -> Option<PathBuf> {
    let kiss_path: Vec<&String> = if let Some(search_path) = search_path {
        vec![search_path]
    } else {
        Vec::from_iter(config.kiss_path.iter())
    };

    // Use Rayon to parallelize the iteration through kiss_path directories
    let found_packages: Vec<PathBuf> = iter!(kiss_path)
        .flat_map(|path| read_a_dir_and_sort(path.as_str(), false, &[]))
        .filter(|package| {
            let package_name = match package.file_name() {
                Some(file_name) => file_name.to_string_lossy(),
                _ => std::borrow::Cow::Borrowed(""),
            };

            name == package_name
        })
        .collect();

    if !found_packages.is_empty() {
        found_packages.first().cloned()
    } else {
        None
    }
}

pub fn pkg_find(config: &Config, name: &str, version: bool, recursive: bool) {
    // Use Rayon to parallelize the iteration through kiss_path directories
    let found_packages: Vec<PathBuf> = iter!(config.kiss_path)
        .flat_map(|path| read_a_dir_and_sort(path.as_str(), false, &[]))
        .filter(|package| {
            let package_name = match package.file_name() {
                Some(file_name) => file_name.to_string_lossy(),
                _ => std::borrow::Cow::Borrowed(""),
            };

            (recursive && package_name.contains(name)) || name == package_name
        })
        .collect();

    if !found_packages.is_empty() {
        for package in &found_packages {
            if version {
                if let Some(ver) = extract_package_version(package) {
                    println!("{}:{}", package.to_string_lossy(), ver);
                } else {
                    println!("{}", package.to_string_lossy());
                }
            } else {
                println!("{}", package.to_string_lossy());
            }
        }
    }
}

fn extract_package_version(package: &Path) -> Option<String> {
    let version_path: PathBuf = package.join("version");

    if version_path.exists() {
        let mut version_lines: Vec<String> =
            read_a_files_lines(&version_path).unwrap_or_else(|_| {
                panic!(
                    "Failed to read version file ({})",
                    version_path.to_string_lossy()
                )
            });

        if let Some(version_line) = version_lines.pop() {
            let mut version_parts = version_line.split_whitespace();
            if let Some(ver_pre) = version_parts.next() {
                if let Some(rel_pre) = version_parts.next() {
                    return Some(format!("{}-{}", ver_pre, rel_pre));
                }
            }
        }
    }

    None
}

pub fn pkg_cache(config: &Config, pkg: &str) -> Option<String> {
    let version: String = pkg_find_version(config, pkg, None)
        .unwrap_or_else(|| die!(pkg.to_owned() + ":", "Failed to get version"));

    let file: String = format!(
        "{}/{}@{}.tar.",
        config.bin_dir.to_string_lossy(),
        pkg,
        version
    );
    let file_with_ext: String = format!("{}{}", file, config.kiss_compress);

    if Path::new(file_with_ext.as_str()).exists() {
        return Some(file_with_ext);
    } else {
        for entry in fs::read_dir(&config.bin_dir).expect("Failed to read BIN_DIR") {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        if file_name_str.starts_with(file.as_str()) {
                            return Some(file_name_str.to_owned());
                        }
                    }
                }
            }
        }
    }

    None
}
