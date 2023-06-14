use seahorse::Context;

use std::path::Path;

// global variables
use super::KISS_PATH;

use super::get_args;
use super::read_a_dir_and_sort;

use super::read_a_files_lines;

pub fn find_pkg_version(name: &str) -> String {
    let package = find_pkg(name, false);
    let package_path: &Path = Path::new(&package);
    let version_path = package_path.join("version");

    if version_path.exists() {
	let mut version: Vec<String> = read_a_files_lines(&version_path, format!("Failed to read version file ({})", version_path.display()).as_str());

	// part version and release
	version = version.first().unwrap().to_owned().split(" ").map(|e| e.to_owned()).collect();

	// first element is version
	let ver_pre = version.clone().into_iter().nth(0).unwrap();
	// second element is release
	let rel_pre = version.clone().into_iter().nth(1).unwrap();

	println!("{}:{}-{}", package_path.display(), ver_pre, rel_pre);
    }
    name.to_owned()
}

pub fn find_pkg(name: &str, print: bool) -> String {
    let kiss_path = &*KISS_PATH;

    let mut wanted_package: String = String::new();

    for path in kiss_path {
	let packages: Vec<_> = read_a_dir_and_sort(path);
	for package in packages {
	    let package_path = package.path();
	    let package_name = package_path.file_name().unwrap().to_str().unwrap();
	    // find packages and print
	    if print && package_name.contains(name) {
		println!("{}", package_path.display());
	    }
	    // find the first package that matches in KISS_PATH and break the loop
	    else if !print && name.to_owned().contains(package_name) {
		wanted_package = format!("{}", package_path.display());
		break
	    }
	}
    }

    wanted_package
}

pub fn search_action(c: &Context) {
    let search: Vec<&str> = get_args(&c);

    // search package
    for package in search {
	find_pkg(package, true);
    }
}
