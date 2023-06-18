// cli
use seahorse::Context;
use super::get_args;

use super::search::{pkg_find, pkg_find_version};
use super::source::{pkg_source, pkg_source_resolve, pkg_source_tar};
use super::manifest::pkg_manifest;

use super::get_repo_dir;
use super::get_repo_name;

use super::read_a_files_lines;
use super::remove_chars_after_last;
use super::mkcd;
use super::copy_folder;

// manage global variables
use super::{get_deps, add_dep};
use super::{get_explicit, add_explicit, remove_explicit};
use super::{SYS_DB, PKG_DB};
use super::{MAK_DIR, PKG_DIR};

use super::{die, log};

use super::set_env_variable_if_undefined;

// std
use std::path::Path;
use std::fs;
// user inout
use std::io::{self, BufRead};
// build
use std::process::{Command, Stdio};

// TODO: finish this function
pub fn pkg_extract(pkg: &str) {
    log(pkg, "Extracting sources");

    let sources_file = format!("{}/sources", get_repo_dir());
    let sources: Vec<String> = read_a_files_lines(sources_file).expect("Failed to read sources file");

    for source in sources {
	let mut source_clone = source.clone();
	let mut dest = String::new();

	// consider user-given folder name
	if source_clone.contains(" ") {
	    let source_parts: Vec<String> = source_clone.split(" ").map(|l| l.to_owned()).collect();
	    source_clone = source_parts.first().unwrap().to_owned();
	    dest = source_parts
		.last()
		.unwrap()
		.to_owned()
		.trim_end_matches('/')
		.to_owned();
	}

	let (res, des) = pkg_source_resolve(source_clone, dest.clone(), false);

	let source_dir: String = format!("{}/{}/{}", *MAK_DIR, pkg, dest.clone());
	// Create the source's directories if not null.
	if !des.is_empty() {
	    mkcd(source_dir.as_str());
	}

	let dest_path = Path::new(source_dir.as_str());

	if res.contains("git+") {
	    copy_folder(Path::new(des.as_str()), &dest_path).expect("Failed to copy git source");
	}
	else if res.contains(".tar") {
	    pkg_source_tar(res);
	}
	else {
	    let file_name = Path::new(res.as_str()).file_name().unwrap();
	    let dest_path = Path::new(source_dir.as_str()).join(file_name);
	    fs::copy(res.clone(), &dest_path).expect("Failed to copy file");
	}
    }
}

// the method we use to store deps and explicit deps is different from original kiss pm.
// we only store implicit deps in DEPS global var and explicit deps in EXPLICIT global var
pub fn pkg_depends(pkg: String, expl: bool, filter: bool, dep_type: String) {
    let deps: Vec<String> = get_deps();
    let explicit: Vec<String> = get_explicit();

    // since pkg_find function sets REPO_DIR and REPO_NAME, run it first
    let pac = pkg_find(pkg.as_str(), false);

    let repo_dir = get_repo_dir();

    // Resolve all dependencies and generate an ordered list. The deepest
    // dependencies are listed first and then the parents in reverse order.
    if deps.contains(&pkg) {
	return;
    }

    if filter == false || explicit.contains(&pkg) || Path::new(&*SYS_DB).join(pkg.clone()).exists() {
	return;
    }

    if !pac.is_empty() || Path::new(&repo_dir).join("depends").exists() {
	let repo_dir = get_repo_dir();
	let depends = read_a_files_lines(format!("{}/depends", repo_dir)).unwrap();
	for dependency in depends {
	    let mut dep = dependency.clone();
	    if dependency.starts_with('#') {
		continue
	    }

	    let mut dependency_type: String = String::new();
	    if dependency.contains(" make") {
		dependency_type = "make".to_owned();
		dep = remove_chars_after_last(&dependency, ' ').trim_end().to_owned();
	    }

	    pkg_depends(dep.clone(), false, filter, dependency_type);
	}
    } else {
	die(pac.as_str(), "not found");
    }

    // TODO: add pkg_cache to condition
    if !expl || dep_type == "make" {
	add_dep(pkg);
    }

    // # Add parent to dependencies list.
    // if ! equ "$2" expl || { equ "$5" make && ! pkg_cache "$1"; }; then
    //     deps="$deps $1"
    // fi

}

pub fn pkg_build_all(packages: Vec<&str>) {
    // find dependencies
    if !packages.is_empty() {
        for package in packages {
	    pkg_depends(package.to_owned(), true, true, String::new());
	    add_explicit(package.to_owned());
        }
    } else {
	let package = get_repo_name();
        pkg_depends(package.clone(), true, true, String::new());
	add_explicit(package);
    }

    let deps = get_deps();

    // If an explicit package is a dependency of another explicit package,
    // remove it from the explicit list.
    for package in get_explicit() {
	if deps.contains(&package) {
	    remove_explicit(package)
	}
    }

    let explicit = get_explicit();

    // log
    let mut implicit_text: String = String::new();
    if !deps.is_empty() {
	implicit_text = format!(", implicit: {}", deps.join(" "));
    }
    log("Building:", format!("explicit: {}{}", explicit.join(" "), implicit_text).as_str());

    if !deps.is_empty() {
	// Ask for confirmation if extra packages need to be built.
	log("Continue?:", "Press Enter to continue or Ctrl+C to abort");

	// get user input
	io::stdin().lock().lines().next();
    }

    // TOOD: add check for prebuilt dependencies
    // for package in ...

    let all_packages = deps.iter().chain(explicit.iter());

    let package_count: usize = all_packages.clone().count();

    for package in all_packages.clone() {
	pkg_source(package, false, true);

	// TODO: add pkg_verify function and complete this code
	// ! [ -f "$repo_dir/sources" ] || pkg_verify "$pkg"
    }

    let mut build_cur: usize = 0;

    for package in all_packages {
	// print status
	build_cur += 1;
	let build_status: String = format!("Building package ({}/{})", build_cur, package_count);
	log(package, build_status.as_str());

	pkg_find_version(package, false);

	let repo_dir = get_repo_dir();

	if Path::new(repo_dir.as_str()).join("sources").exists() {
	    pkg_extract(package);
	}

	pkg_build(package);

	pkg_manifest(package);
    }
}

pub fn pkg_build(pkg: &str) {
    mkcd(format!("{}/{}", *MAK_DIR, pkg).as_str());

    log(pkg, "Starting build");

    set_env_variable_if_undefined("AR", "ar");
    set_env_variable_if_undefined("CC", "cc");
    set_env_variable_if_undefined("CXX", "c++");
    set_env_variable_if_undefined("NM", "nm");
    set_env_variable_if_undefined("RANLIB", "ranlib");

    let executable = format!("{}/build", get_repo_dir());
    let install_dir = format!("{}/{}", *PKG_DIR, pkg);
    let mut child = Command::new(executable)
        .arg(install_dir)
        .stdout(Stdio::inherit())
        .spawn()
        .expect("Failed to execute build file");

    // wait for build to finish
    let status = child.wait().expect("Failed to wait for command");
    if status.success() {

	// Copy the repository files to the package directory.
	let pkg_db_dir = format!("{}/{package_name}/{}/{package_name}", *PKG_DIR, PKG_DB, package_name = pkg);
	mkcd(pkg_db_dir.as_str());
	copy_folder(Path::new(get_repo_dir().as_str()), Path::new(pkg_db_dir.as_str())).expect("Failed to copy repository files to package directory");

	// give info
	log(pkg, "Successfully built package")
    } else {
	die(pkg, "Build failed")
    }

}

pub fn build_action(c: &Context) {
    let packages: Vec<&str> = get_args(&c);

    pkg_build_all(packages)
}
