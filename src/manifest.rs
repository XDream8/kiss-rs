use super::{PKG_DB};
use super::{PKG_DIR, TMP_DIR};

use super::read_a_dir_and_sort;

// file libs
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

// logging
use super::log;

pub fn pkg_manifest(pkg: &str) {
    log(pkg, "Generating manifest");

    // tmp file
    let tmp_file_name = format!("{}-manifest", pkg);
    let tmp_file_path = Path::new(&*TMP_DIR).join(tmp_file_name);
    let mut tmp_file = File::create(&tmp_file_path).expect("Failed to create tmp file");

    // Create a list of all files and directories. Append '/' to the end of
    // directories so they can be easily filtered out later. Also filter out
    // all libtool .la files and charset.alias.
    let pkg_dir = format!("{}/{}", *PKG_DIR, pkg);
    // this will be added to manifest
    let pkg_manifest_pathbuf = PathBuf::from(format!("{}/{package_name}/{}/{package_name}/manifest", *PKG_DIR, PKG_DB, package_name = pkg).as_str());
    // prefix that will be removed from manifest entries
    let prefix: &str= pkg_dir.as_str();

    // remove prefix
    let mut manifest: Vec<PathBuf> = std::iter::once(pkg_manifest_pathbuf.to_path_buf())
	.chain(read_a_dir_and_sort(pkg_dir.as_str(), true))
	.filter_map(|path| {
	    // in case if there is a "//" in path
	    let path_str = path.to_string_lossy().replace("//", "/");
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
    std::fs::copy(tmp_file_path.to_string_lossy().into_owned(), pkg_manifest_pathbuf)
	.expect("Failed to move tmp_file");
}
