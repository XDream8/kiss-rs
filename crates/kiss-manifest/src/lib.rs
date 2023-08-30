use shared_lib::globals::Config;

use shared_lib::{is_symlink, read_a_dir_and_sort, read_a_files_lines, tmp_file};

// logging
use shared_lib::signal::pkg_clean;
use shared_lib::{die, log};

// libs
use std::io::Write;
use std::path::{Path, PathBuf};

// threading
#[cfg(feature = "threading")]
use rayon::iter::ParallelIterator;
use shared_lib::{iter, sort_reverse};

pub fn pkg_manifest(config: &Config, pkg: &str, dir: &Path) {
    log!(pkg, "Generating manifest");

    let (mut tmp_file, tmp_file_path) =
        tmp_file(config, pkg, "manifest").expect("Failed to create tmp_file");

    // Create a list of all files and directories. Append '/' to the end of
    // directories so they can be easily filtered out later. Also filter out
    // all libtool .la files and charset.alias.
    // this will be added to manifest
    let pkg_manifest_pathbuf: PathBuf = dir
        .join(pkg)
        .join(&config.pkg_db)
        .join(pkg)
        .join("manifest");
    // pkg_dir/prefix - this will be removed from manifest entries
    let prefix: String = format!("{}/{}", dir.to_string_lossy(), pkg);

    // remove manifest file if it already exists
    if pkg_manifest_pathbuf.exists() {
        std::fs::remove_file(&pkg_manifest_pathbuf)
            .expect("Failed to remove already existing manifest file");
    }

    // read contents of directory
    let mut files: Vec<PathBuf> =
        read_a_dir_and_sort(prefix.as_str(), true, &[".la", "charset.alias"]);
    files.push(pkg_manifest_pathbuf.to_owned());

    // remove prefix
    let mut manifest: Vec<PathBuf> = iter!(files)
        .filter_map(|path| {
            let path_str = path.to_string_lossy().to_string();
            let modified_path = PathBuf::from(&path_str);

            if path_str.starts_with(&prefix) {
                if modified_path.is_dir() {
                    // add ’/’ to end of the directories and strip prefix
                    let mut path_buf = modified_path;
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
    sort_reverse!(manifest);

    for file in manifest {
        tmp_file
            .write_all(file.into_os_string().into_string().unwrap().as_bytes())
            .expect("Failed to write manifest to tmp file");
        tmp_file
            .write_all(b"\n")
            .expect("Failed to write manifest to tmp file");
    }

    // copy manifest file to actual dest
    std::fs::copy(tmp_file_path, pkg_manifest_pathbuf)
        .expect("Failed to copy tmp_file to actual manifest path");
}

pub fn pkg_manifest_validate(config: &Config, pkg: &str, path: &str, manifest_path: &PathBuf) {
    // debug comes from caller
    if config.debug || config.verbose {
        log!(pkg, "Checking if manifest is valid");
    }

    let relative_manifest_elements: Vec<String> = read_a_files_lines(manifest_path)
        .expect("Failed to read manifest file")
        .iter()
        .map(|line| line.trim_start_matches('/').to_string())
        .collect();

    let count: usize = iter!(relative_manifest_elements)
        .map(|line| {
            let element_path: PathBuf = Path::new(path).join(line.as_str());

            if !element_path.exists() && !is_symlink(element_path.as_path()) {
                println!("{}", line);
                1
            } else {
                0
            }
        })
        .sum();

    if count != 0 {
        die!(pkg, "manifest contains", count, "non-existent files");
    }
}
