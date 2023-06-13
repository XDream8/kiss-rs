use seahorse::Context;

use std::env;

// using this to remove duplicate path entries
use std::collections::HashSet;

use rayon::prelude::*;

use super::get_args;
use super::read_a_dir_and_sort;
use super::INSTALLED_DIR;

pub fn find_pkg(name: &str) {
    // get output of KISS_PATH environment variable
    let path_var: String = match env::var("KISS_PATH") {
	Ok(v) => v,
	_ => INSTALLED_DIR.to_owned()
    };

    // create paths vector
    let binding = path_var.split(":");
    let mut paths: Vec<&str> = binding.collect();

    // add installed packages directory
    paths.push(INSTALLED_DIR);

    // remove duplicates from paths
    let mut set = HashSet::new();
    paths.retain(|x| set.insert(x.clone()));

    for path in paths {
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
