use std::path::PathBuf;

use crate::error::Error;
use crate::source::{parse_source_line, pkg_is_binary_available, Source};

#[derive(Default, Debug, Eq, Hash, PartialEq, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub sources: Vec<Source>,
    pub package_repo_path: PathBuf,
    pub cached_binary: Option<PathBuf>,
}

pub fn pkg_info_from_path() {}

pub fn pkg_installed_get_info(
    query: &String,
    sys_package_database: &PathBuf,
) -> Result<Package, Error> {
    let package_path = sys_package_database.join(query);
    if package_path.exists() {
        let version_file: PathBuf = package_path.join("version");
        let sources_file: PathBuf = package_path.join("sources");

        let version: String = std::fs::read_to_string(&version_file)?
            .trim()
            .replace(' ', "-");
        let sources: Vec<Source> = std::fs::read_to_string(&sources_file)?
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| parse_source_line(line))
            .flatten()
            .collect::<Vec<_>>();

        Ok(Package {
            name: query.to_string(),
            version: (&version.trim()).to_string(),
            sources,
            package_repo_path: package_path,
            cached_binary: None,
        })
    } else {
        Err(Error::PackageIsNotInstalled)
    }
}
