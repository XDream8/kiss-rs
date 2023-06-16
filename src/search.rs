use seahorse::Context;

use std::path::Path;

// global variables
use super::KISS_PATH;
use super::SYS_DB;

use super::get_args;
use super::read_a_dir_and_sort;
use super::read_a_files_lines;

use super::set_repo_dir;
use super::{get_repo_name, set_repo_name};

use super::get_directory_name;

use super::die;

pub fn pkg_find_version(name: &str, print: bool) -> String {
    let package = pkg_find(name, print);
    let package_path: &Path = Path::new(&package);
    let version_path = package_path.join("version");

    if version_path.exists() {
        let mut version: Vec<String> = read_a_files_lines(
            &version_path,
            format!("Failed to read version file ({})", version_path.display()).as_str(),
        );

        // part version and release
        version = version
            .first()
            .unwrap()
            .to_owned()
            .split(" ")
            .map(|e| e.to_owned())
            .collect();

        // first element is version
        let ver_pre = version.clone().into_iter().nth(0).unwrap();
        // second element is release
        let rel_pre = version.clone().into_iter().nth(1).unwrap();

        if print {
            println!("{}:{}-{}", package_path.display(), ver_pre, rel_pre);
        }
    }
    name.to_owned()
}

pub fn pkg_find(name: &str, print: bool) -> String {
    let mut kiss_path: Vec<String> = KISS_PATH.to_vec();

    let mut wanted_package: String = String::new();

    // remove SYS_DB path if we call this function from another function
    // checksum etc.
    if !print {
        kiss_path.retain(|x| x != SYS_DB);
    }

    for path in kiss_path {
        let packages: Vec<_> = read_a_dir_and_sort(path.as_str());
        for package in packages {
            let package_path = package.path();
            let package_name = package_path.file_name().unwrap().to_str().unwrap();
            // find packages and print
            if print && package_name.contains(name) {
                println!("{}", package_path.display());
            }
            // find the first package that matches in KISS_PATH and break the loop
            else if !print && name.to_owned() == package_name {
                wanted_package = format!("{}", package_path.display());
                break;
            }
        }
    }

    if !wanted_package.is_empty() {
        {
            let wanted_package_clone = wanted_package.clone();

            let repo_name: String = get_repo_name();

            let directory_name = get_directory_name(&wanted_package_clone);

            set_repo_dir(wanted_package.clone());
            set_repo_name(directory_name.to_owned());

            if repo_name.is_empty() {
                die(&wanted_package.clone(), "Unable to get directory name");
            }
        }
    }

    wanted_package
}

pub fn search_action(c: &Context) {
    let search: Vec<&str> = get_args(&c);

    // search package
    for package in search {
        pkg_find(package, true);
    }
}
