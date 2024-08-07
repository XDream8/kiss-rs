use std::path::{Path, PathBuf};

use crate::compression::CompressionType;
use crate::error::Error;
use crate::pkg::{extract_package_sources, extract_package_version};
use crate::source::{pkg_is_binary_available, Source};

/// we may make package struct include build file path in the future but it is not really neccessary
/// since there is already `package_repo_path` in the struct. so users of this api could just do:
/// package_repo_path.join("build")
#[derive(Default, Debug, Eq, Hash, PartialEq, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub sources: Vec<Source>,
    pub package_repo_path: PathBuf,
    pub cached_binary: Option<PathBuf>,
}

pub fn pkg_info_from_path(
    package_name: &String,
    package_path: &Path,
    source_cache_dir: Option<&PathBuf>,
    binary_cache_dir: Option<&PathBuf>,
    compression_type: Option<&CompressionType>,
) -> Result<Package, Error> {
    let version: String = extract_package_version(package_path)?;
    let sources: Vec<Source> =
        extract_package_sources(package_path, package_name, source_cache_dir)?;

    let cached_binary = if let Some(binary_cache_dir) = binary_cache_dir {
        if let Some(compression_type) = compression_type {
            pkg_is_binary_available(binary_cache_dir, package_name, &version, &compression_type)
        } else {
            None
        }
    } else {
        None
    };

    Ok(Package {
        name: package_name.to_string(),
        version: (&version.trim()).to_string(),
        sources,
        package_repo_path: package_path.to_path_buf(),
        cached_binary,
    })
}

pub fn pkg_get_info(
    query: &String,
    source_cache_dir: Option<&PathBuf>,
    binary_cache_dir: Option<&PathBuf>,
    compression_type: Option<&CompressionType>,
    repositories: &Vec<PathBuf>,
) -> Result<Package, Error> {
    for repository_path in repositories {
        let package_path: PathBuf = repository_path.join(query);
        if package_path.exists() {
            return pkg_info_from_path(
                query,
                &package_path,
                source_cache_dir,
                binary_cache_dir,
                compression_type,
            );
        }
    }

    Err(Error::PackageNotFound)
}

// this does not check if binary is cached since this is for installed packages
pub fn pkg_installed_get_info(
    query: &String,
    sys_package_database: &Path,
    source_cache_dir: Option<&PathBuf>,
) -> Result<Package, Error> {
    let package_path = sys_package_database.join(query);
    if package_path.exists() {
        pkg_info_from_path(query, &package_path, source_cache_dir, None, None)
    } else {
        Err(Error::PackageIsNotInstalled)
    }
}
