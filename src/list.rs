use seahorse::Context;

use std::path::Path;

use super::cat;
use super::get_args;
use super::read_a_dir_and_sort;
use super::SYS_DB;

use super::die;

pub fn list_action(c: &Context) {
    let search: Vec<&str> = get_args(&c);

    if search.is_empty() {
        // get installed packages
        let installed_packages: Vec<_> = read_a_dir_and_sort(&*SYS_DB, false);

        for package in installed_packages {
            let version: String = cat(&package.join("version"))
		.unwrap()
		.replace(" ", "-")
		.replace("\n", "");
	    println!(
		"{} {}",
		package.file_name().unwrap().to_str().unwrap(),
		version
	    )
        }
    } else {
        for package in search {
            let path: &Path = &Path::new(&*SYS_DB).join(&package);
            if path.exists() {
                let version: String = cat(&path.join("version"))
                    .unwrap()
                    .replace(" ", "-")
                    .replace("\n", "");
                println!(
                    "{} {}",
                    path.file_name().unwrap().to_str().unwrap(),
                    version
                )
            } else {
                die!(package, "not found");
            }
        }
    }
}
