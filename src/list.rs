use seahorse::Context;

use colored::*;

use std::path::Path;

use std::process::exit;

use super::cat;
use super::read_a_dir_and_sort;
use super::INSTALLED_DIR;

pub fn list_action(c: &Context) {
    let mut search: Vec<String> = vec![];

    for package in &c.args {
	search.push(format!("{}", package).to_owned())
    }

    if search.is_empty() {
	// get installed packages
	let installed_packages: Vec<_> = read_a_dir_and_sort(INSTALLED_DIR);

	for package in installed_packages {
	    let version: String = cat(&package.path().join("version")).unwrap().replace(" ", "-").replace("\n", "");
	    println!("{} {}", package.path().file_name().unwrap().to_str().unwrap(), version)
	}
    } else {
	for package in search {
	    let path: &Path = &Path::new(INSTALLED_DIR).join(&package);
	    if path.exists() {
		let version: String = cat(&path.join("version")).unwrap().replace(" ", "-").replace("\n", "");
		println!("{} {}", path.file_name().unwrap().to_str().unwrap(), version)
	    }
	    else {
		eprintln!("{} '{}' not found", "ERROR".yellow(), package);
		exit(1);
	    }
	}
    }
}
