use crate::{
    common_funcs::{cat, read_a_dir_and_sort},
    source::{parse_source_line, Source},
};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use crate::error::Error;

pub fn pkg_is_installed(query: &String, sys_package_database: &Path) -> bool {
    let package_path = sys_package_database.join(query);
    package_path.exists()
}

// spent some time optimizing this to be sooo fast
pub fn pkg_print_installed_packages(
    search_query: Vec<String>,
    sys_package_database: &PathBuf,
    version_param: bool,
) -> Result<(), Error> {
    if search_query.is_empty() {
        // get installed packages
        read_a_dir_and_sort(sys_package_database, false)?
            .iter()
            .flat_map(|package| {
                let file_name = package.file_name().unwrap_or(OsStr::new(""));
                if version_param {
                    let version_path = package.join("version");
                    let version = cat(&version_path).ok()?.replace(' ', "-").replace('\n', "");
                    Some(format!("{} {}", file_name.to_string_lossy(), version))
                } else {
                    Some(file_name.to_string_lossy().into_owned())
                }
            })
            .for_each(|package| println!("{}", package));
    } else {
        for package in &search_query {
            let package_path = sys_package_database.join(package);
            match package_path.exists() {
                true => {
                    if version_param {
                        let version = extract_package_version(&package_path)?;
                        println!("{} {}", package, version);
                    } else {
                        println!("{}", package);
                    }
                }
                false => {
                    eprintln!("{} not found", package);
                    return Err(Error::PackageNotFound);
                }
            }
        }
    }

    Ok(())
}

pub fn pkg_find_and_print(
    kiss_path: &Vec<PathBuf>,
    name: &str,
    recursive: bool,
    version: bool,
) -> Result<(), Error> {
    for path in kiss_path {
        if let Ok(entries) = fs::read_dir(path) {
            let found_packages: Vec<PathBuf> = entries
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|package| {
                    let package_name = match package.file_name() {
                        Some(file_name) => file_name.to_string_lossy(),
                        _ => std::borrow::Cow::Borrowed(""),
                    };

                    (recursive && package_name.contains(name)) || name == package_name
                })
                .collect();

            found_packages.iter().for_each(|package| {
                if version {
                    if let Ok(version) = extract_package_version(package) {
                        println!("{}:{}", package.to_string_lossy(), version);
                    }
                } else {
                    println!("{}", package.to_string_lossy());
                }
            })
        }
    }

    Ok(())
}

pub fn extract_package_version(package_path: &Path) -> Result<String, Error> {
    let version_file_path = package_path.join("version");

    if version_file_path.exists() {
        Ok(fs::read_to_string(version_file_path)?
            .trim()
            .replace(' ', "-")
            .to_owned())
    } else {
        Err(Error::VersionFileNotFound)
    }
}

pub fn extract_package_sources(
    package_path: &Path,
    package_name: &String,
    source_cache_dir: Option<&PathBuf>,
) -> Result<Vec<Source>, Error> {
    let sources_file_path: PathBuf = package_path.join("sources");

    if sources_file_path.exists() {
        Ok(std::fs::read_to_string(sources_file_path)?
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                parse_source_line(line, package_name, Some(package_path), source_cache_dir)
            })
            .collect::<Vec<_>>())
    } else {
        Err(Error::SourcesFileNotFound)
    }
}

pub fn pkg_find_path(name: &str, repo_paths: &[PathBuf]) -> Option<PathBuf> {
    repo_paths.iter().find_map(|repo_path| {
        let pkg_path = repo_path.join(name);
        if pkg_path.is_dir() {
            Some(pkg_path)
        } else {
            None
        }
    })
}
