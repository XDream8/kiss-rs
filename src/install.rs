// cli
use seahorse::Context;
use super::get_args;

use super::manifest::pkg_manifest_validate;
use super::search::pkg_find_version;
use super::source::pkg_source_tar;

use super::{PKG_DB, SYS_DB};
use super::{BIN_DIR, TAR_DIR};
use super::{KISS_COMPRESS, KISS_FORCE};

use super::{log, die};

use super::get_repo_name;
use super::mkcd;
use super::read_a_files_lines;
use super::remove_chars_after_last;

use super::tmp_file;

use std::fs;
use std::path::{Path, PathBuf};

pub fn pkg_cache(pkg: &str) -> Option<String> {
    let version: String = pkg_find_version(pkg, false);

    let file: String = format!("{}/{}@{}.tar.", *BIN_DIR, pkg, version);
    let file_with_ext = format!("{}{}", file, *KISS_COMPRESS);

    if Path::new(file_with_ext.as_str()).exists() {
	return Some(file_with_ext);
    } else {
	for entry in fs::read_dir(&*BIN_DIR).expect("Failed to read BIN_DIR") {
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

pub fn pkg_conflicts(pkg: &str) {
    log!(pkg, "Checking for package conflicts");

    let (tmp_manifest_files, tmp_manifest_files_path) = tmp_file(pkg, "manifest-files").expect("Failed to create tmp_manifest_files");
    let (tmp_found_conflicts, tmp_found_conflicts_path) = tmp_file(pkg, "found-conflicts").expect("Failed to create tmp_found_conflicts");



}

pub fn pkg_installable(pkg: &str, depends_file_path: String) {
    log!(pkg, "Checking if package installable");

    let mut count: usize = 0;

    let depends = read_a_files_lines(depends_file_path).unwrap();

    for dependency in depends {
	let mut dep = dependency.clone();
	if dependency.starts_with('#') {
	    continue
	}

	let mut dependency_type: String = String::new();
	if dependency.contains(" make") {
	    dependency_type = "make".to_owned();
	    dep = remove_chars_after_last(&dependency, ' ').trim_end().to_owned();
	}

	if Path::new(&*SYS_DB).join(dep.clone()).exists() {
	    continue
	}

	println!("{} {}", dep, dependency_type);

	count +=1;
    }

    if count != 0 {
	die!(pkg, format!("Package not installable, missing {} package(s)", count));
    }
}

pub fn pkg_install(package_tar: &str, force: bool) {
    // pkg name to be used
    let mut pkg: String = String::new();
    // pkg tarball to be used
    let mut tar_file: String = package_tar.to_owned();

    if package_tar.contains(".tar.") {
	// remove everything before the last ’/’ and everything after the ’@’ char
	pkg = package_tar.rsplitn(2, '/').next().unwrap().split('@').next().unwrap().to_owned();
    } else {
	if let Some(tarball) = pkg_cache(package_tar) {
	    tar_file = tarball;
	} else {
	    die!(package_tar, "Not yet built");
	}

	pkg = package_tar.to_owned();
    }

    // cd into extract directory
    let extract_dir: String = format!("{}/{}", *TAR_DIR, pkg);
    mkcd(extract_dir.as_str());

    // extract to current dir
    pkg_source_tar(tar_file, false);

    let manifest_path: PathBuf = Path::new(format!("{}/{}", extract_dir, &*PKG_DB).replace("//", "/").as_str()).join(pkg.as_str()).join("manifest");
    if !manifest_path.exists() {
	println!("{}, {}", extract_dir, manifest_path.display());
	die!("", "Not a valid KISS package");
    }

    if force == true || *KISS_FORCE != "1" {
	pkg_manifest_validate(pkg.as_str(), extract_dir.as_str(), manifest_path);
	pkg_installable(pkg.as_str(), format!("./{}/{}/depends", &*PKG_DB, pkg));
    }

    pkg_conflicts(pkg.as_str());
}

pub fn install_action(c: &Context) {
    let packages: Vec<&str> = get_args(&c);

    // search package
    if !packages.is_empty() {
        for package in packages {
            pkg_install(package, false);
        }
    } else {
        pkg_install(get_repo_name().as_str(), false);
    }
}
