// cli
use crate::read_a_dir_and_sort;
use seahorse::Context;

use super::get_args;

use super::manifest::{pkg_manifest, pkg_manifest_validate};
use super::search::pkg_find_version;
use super::source::pkg_source_tar;

use super::{BIN_DIR, TAR_DIR};
use super::{CHO_DB, PKG_DB, SYS_DB};
use super::{KISS_CHOICE, KISS_COMPRESS, KISS_FORCE};

use super::{die, log};

use super::get_repo_name;
use super::mkcd;
use super::read_a_files_lines;
use super::remove_chars_after_last;

// for checking conflicts
use super::resolve_path;
use std::io::{BufRead, BufReader};

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

// TODO: improve performance - it must be slow atm
pub fn pkg_conflicts(pkg: &str, manifest_file_path: &PathBuf) -> Result<(), std::io::Error> {
    log!(pkg, "Checking for package conflicts");

    let mut resolved_paths: Vec<String> = Vec::new();
    let mut conflicts: Vec<String> = Vec::new();

    let manifest_contents: Vec<String> = read_a_files_lines(manifest_file_path)?;
    for line in manifest_contents {
        // store absolute paths in vector
        if line.ends_with('/') {
            continue;
        }
        if let Some(resolved_path) = resolve_path(line.as_str()) {
            resolved_paths.push(format!("{}", resolved_path.display()));
        }
    }

    // only get manifest files
    let sys_manifest_files: Vec<PathBuf> = read_a_dir_and_sort(&SYS_DB, true)
        .iter()
        .filter(|file| file.file_name().unwrap().to_str() == Some("manifest"))
        .map(|name| name.to_path_buf())
        .collect();

    let mut conflicts_found = false;
    let mut safe = false;

    for sys_manifest_path in sys_manifest_files {
        // do not check against the package manifest, if package wanted to be installed is already installed
        if sys_manifest_path
            .to_string_lossy()
            .to_string()
            .contains(PKG_DB)
        {
            continue;
        };

        let sys_manifest_file = fs::File::open(sys_manifest_path).unwrap();
        let sys_manifest_reader = BufReader::new(sys_manifest_file);

        for line in sys_manifest_reader.lines().flatten() {
            let found = resolved_paths.iter().any(|path| path == &line);
            if found {
                conflicts_found = true;
                conflicts.push(line);
                break;
            }
        }

        if conflicts_found {
            break;
        }
    }

    // TODO: Enable alternatives automatically if it is safe to do so.
    // This checks to see that the package that is about to be installed
    // doesn't overwrite anything it shouldn't in '/var/db/kiss/installed'.
    if !conflicts.is_empty() {
        safe = true;
    }

    if *KISS_CHOICE == "1" && safe && conflicts_found {
        // Handle conflicts and create choices
        let choice_directory_path: String = format!("{}/{}/{}", &*TAR_DIR, pkg, CHO_DB);
        let choice_directory = Path::new(choice_directory_path.as_str());
        // Create the "choices" directory inside of the tarball.
        // This directory will store the conflicting file.
        fs::create_dir_all(choice_directory)?;

        let mut choices_created: usize = 0;

        for conflict in conflicts {
            println!("Found conflict: {}", conflict);

            let new_file_name: String = format!(
                "{}>{}",
                pkg,
                conflict.trim_start_matches('/').replace('/', ">")
            );
            let choice_file_path: String = format!("{}/{}", choice_directory_path, new_file_name);
            let real_conflict_path: String = format!("{}/{}/{}", &*TAR_DIR, pkg, conflict);

            fs::rename(real_conflict_path, choice_file_path)?;
            choices_created += 1;
        }

        if choices_created > 0 {
            log!(pkg, "Converted all conflicts to choices (kiss a)");
            // Rewrite the package's manifest to update its location
            // to its new spot (and name) in the choices directory.
            pkg_manifest(pkg, &TAR_DIR);
        }
    } else if conflicts_found {
        println!("Package '{}' conflicts with another package !>", pkg);
        println!("Run 'KISS_CHOICE=1 kiss i '{}' to add conflicts !>", pkg);
        die!("", "as alternatives. !>");
    }

    Ok(())
}

pub fn pkg_installable(pkg: &str, depends_file_path: String) {
    log!(pkg, "Checking if package installable");

    let mut count: usize = 0;

    let depends = read_a_files_lines(depends_file_path).unwrap();

    for dependency in depends {
        let mut dep = dependency.clone();
        if dependency.starts_with('#') {
            continue;
        }

        let mut dependency_type: String = String::new();
        if dependency.contains(" make") {
            dependency_type = "make".to_owned();
            dep = remove_chars_after_last(&dependency, ' ')
                .trim_end()
                .to_owned();
        }

        if Path::new(&*SYS_DB).join(dep.clone()).exists() {
            continue;
        }

        println!("{} {}", dep, dependency_type);

        count += 1;
    }

    if count != 0 {
        die!(pkg, "Package not installable, missing {} package(s)", count);
    }
}

pub fn pkg_install(package_tar: &str, force: bool) {
    // pkg name to be used
    let mut pkg: String = String::new();
    // pkg tarball to be used
    let mut tar_file: String = package_tar.to_owned();

    if package_tar.contains(".tar.") {
        // remove everything before the last ’/’ and everything after the ’@’ char
        pkg = package_tar
            .rsplit('/')
            .next()
            .unwrap()
            .split('@')
            .next()
            .unwrap()
            .to_owned();
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
    pkg_source_tar(tar_file.clone(), false);

    let manifest_path: PathBuf = Path::new(
        format!("{}/{}", extract_dir, PKG_DB)
            .replace("//", "/")
            .as_str(),
    )
    .join(pkg.as_str())
    .join("manifest");
    if !manifest_path.exists() {
        println!("{}, {}", extract_dir, manifest_path.display());
        die!("", "Not a valid KISS package");
    }

    if !force || *KISS_FORCE != "1" {
        pkg_manifest_validate(pkg.as_str(), extract_dir.as_str(), manifest_path.clone());
        pkg_installable(pkg.as_str(), format!("./{}/{}/depends", PKG_DB, pkg));
    }

    pkg_conflicts(pkg.as_str(), &manifest_path).expect("Failed to check conflicts");

    log!(
        pkg,
        "Installing package ({})",
        tar_file
            .split('/')
            .last()
            .expect("Failed to get tar_file name")
    );

    // If the package is already installed (and this is an upgrade) make a
    // backup of the manifest and etcsums files.
    // let manifest_copy = PathBuf::from(format!("{}/{}", &*SYS_DB, pkg)).join("manifest-copy");
    // let etcsums_copy = PathBuf::from(format!("{}/{}", &*SYS_DB, pkg)).join("etcsums-copy");
    // let manifest_diff = PathBuf::from(format!("{}/{}", &*SYS_DB, pkg)).join("manifest-diff");

    // Generate a list of files which exist in the currently installed manifest
    // but not in the newer (to be installed) manifest.
    // let tar_man = Path::new(&*PKG_DB).join(pkg).join("manifest");

    // let tar_man_lines = BufReader::new(fs::File::open(&tar_man).expect("Failed to open manifest file"));
    // let tmp_file_pre_pre_lines = BufReader::new(fs::File::open(&manifest_diff).expect("Failed to open manifest-diff file"));

    // let grep_lines = tmp_file_pre_pre_lines
    //     .lines()
    //     .filter(|line| {
    //         let line = line.as_ref().unwrap();
    //         !tar_man_lines.lines().any(|tar_line| tar_line.as_ref().unwrap() == line)
    //     });

    // Reverse the manifest file so that we start shallow and go deeper as we
    // iterate over each item. This is needed so that directories are created
    // going down the tree.
    // let tar_man_lines_rev: Vec<_> = tar_man_lines.lines().collect();
    // let mut sort_lines_rev = tar_man_lines_rev.clone();
    // sort_lines_rev.sort();

    // let sort_lines = sort_lines_rev.into_iter().map(|line| line.unwrap());

    // if pkg_install_files(Path::new("."), sort_lines, grep_lines).is_ok() &&
    //     pkg_remove_files(tmp_file_pre_lines, tmp_file_pre_pre_lines).is_ok() &&
    //     pkg_install_files(Path::new("."), sort_lines, grep_lines).is_ok()
    // {
    //     log(pkg, "Installed successfully");
    // } else {
    //     log!(pkg, "Failed to install package.");
    //     die!(pkg, "Filesystem now dirty, manual repair needed.");
    // }
}

pub fn install_action(c: &Context) {
    let packages: Vec<&str> = get_args(c);

    if !packages.is_empty() {
        for package in packages {
            pkg_install(package, false);
        }
    } else {
        pkg_install(get_repo_name().as_str(), false);
    }
}
