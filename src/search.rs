use seahorse::Context;

// global variables
use super::KISS_PATH;

use super::get_args;
use super::read_a_dir_and_sort;

pub fn find_pkg(name: &str) {
    let kiss_path = &*KISS_PATH;

    for path in kiss_path {
	let packages: Vec<_> = read_a_dir_and_sort(path);
	for package in packages {
	    let package_path = package.path();
	    let package_name = package_path.file_name().unwrap().to_str().unwrap();
	    if package_name.contains(name) {
		println!("{}", package_path.display());
	    }
	}
    }
}

pub fn search_action(c: &Context) {
    let search: Vec<&str> = get_args(&c);

    // search package
    for package in search {
	find_pkg(package)
    }
}
