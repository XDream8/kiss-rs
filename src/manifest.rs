use super::PKG_DB;

use super::read_a_dir_and_sort;
use super::read_a_files_lines;
use super::tmp_file;

// libs
use std::io::Write;
use std::path::{Path, PathBuf};

// logging
use super::{log, die};

pub fn pkg_manifest(pkg: &str, dir: &str) {
    log!(pkg, "Generating manifest");

    let (mut tmp_file, tmp_file_path) = tmp_file(pkg, "manifest").expect("Failed to create tmp_file");

    // Create a list of all files and directories. Append '/' to the end of
    // directories so they can be easily filtered out later. Also filter out
    // all libtool .la files and charset.alias.
    let pkg_dir = format!("{}/{}", dir, pkg);
    // this will be added to manifest
    let pkg_manifest_pathbuf = PathBuf::from(format!("{}/{package_name}/{}/{package_name}/manifest", dir, PKG_DB, package_name = pkg).as_str());
    // prefix that will be removed from manifest entries
    let prefix: &str= pkg_dir.as_str();

    // remove manifest file if it already exists
    if pkg_manifest_pathbuf.exists() {
	std::fs::remove_file(pkg_manifest_pathbuf.clone()).expect("Failed to remove already existing manifest file");
    }

    // remove prefix
    let mut manifest: Vec<PathBuf> = std::iter::once(pkg_manifest_pathbuf.to_path_buf())
	.chain(read_a_dir_and_sort(pkg_dir.as_str(), true))
	.filter_map(|path| {
	    let path_str = path.to_string_lossy().to_string();
	    let modified_path = PathBuf::from(&path_str);

	    if path_str.starts_with(prefix) {
		if modified_path.is_dir() {
		    // add ’/’ to end of the directories and strip prefix
		    let mut path_buf = modified_path.clone();
		    path_buf.push("");
		    let path_buf_str = path_buf.to_string_lossy();
		    Some(PathBuf::from(&path_buf_str[prefix.len()..]))
		} else {
		    // strip prefix
		    Some(PathBuf::from(&path_str[prefix.len()..]))
		}
	    } else {
		Some(modified_path)
	    }

	})
	.collect();

    // sort manifest reverse alphabetically
    manifest.sort_by(|a, b| b.cmp(a));

    for file in manifest {
	tmp_file.write_all(file.into_os_string().into_string().unwrap().as_bytes()).expect("Failed to write manifest to tmp file");
	tmp_file.write_all(b"\n").expect("Failed to write manifest to tmp file");
    }

    // copy manifest file to actual dest
    std::fs::copy(tmp_file_path, pkg_manifest_pathbuf)
	.expect("Failed to copy tmp_file to actual manifest path");
}

pub fn pkg_manifest_validate(pkg: &str, path: &str, manifest_path: PathBuf) {
    log!(pkg, "Checking if manifest is valid");

    let mut count: usize = 0;

    let manifest_elements: Vec<String> = read_a_files_lines(&manifest_path).expect("Failed to read manifest file");

    for line in manifest_elements {
	let element: String = format!("{}/{}", path, line.clone());

	if !Path::new(element.as_str()).exists() {
	    println!("{}", line);
	    count += 1;
	}
    }

    if count != 0 {
	die!(pkg, format!("manifest contains {} non-existant files", count));
    }
}
