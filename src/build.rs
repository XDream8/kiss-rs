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
use super::{BIN_DIR, MAK_DIR, PKG_DIR};
use super::{KISS_COMPRESS, KISS_STRIP};

use super::{die, log};

use super::set_env_variable_if_undefined;

// std
use std::path::Path;
use std::fs::{self, File};
// user input
use std::io::{self, BufRead, Write};
// strip
use std::io::Read;
// build
use std::process::{Command, Stdio};
// tar
use tar::Builder;
use flate2::write::GzEncoder;
use xz2::write::XzEncoder;
use bzip2::write::BzEncoder;

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

// required for stripping
fn is_matching_directory(path: &Path) -> bool {
    let file_name = path.file_name().unwrap_or_default();
    let parent_dir_name = path.parent().and_then(|p| p.file_name()).and_then(std::ffi::OsStr::to_str);

    let is_sbin = file_name == "sbin";
    let is_bin = file_name == "bin";
    let is_lib = parent_dir_name == Some("lib");

    is_sbin || is_bin || is_lib
}

// for stripping
pub fn strip_files_recursive(directory: &Path) {
    let entries = fs::read_dir(directory).expect("Failed to read directory");

    let lib_and_exec_args: Vec<&str> = vec!("-s", "-R", ".comment", "-R", ".note");
    let object_and_static_lib_args: &str = "-g -R .comment -R .note";

    for entry in entries {
	let entry = entry.unwrap();
	let file_path = entry.path();
	
	if file_path.is_dir() {
	    strip_files_recursive(&file_path);
	}
	else if file_path.is_file() {
	    if let Some(extension) = file_path.extension() {
		if let Some(extension_str) = extension.to_str() {
		    if extension_str == "o" || extension_str == "a" {
			let command = format!("strip {} {}", object_and_static_lib_args, file_path.to_string_lossy());
			println!("{}", command);
			let status = Command::new("strip")
			    .arg(object_and_static_lib_args)
			    .arg(&file_path)
			    .status().expect("Failed to strip file");
			if !status.success() {
			    die(get_repo_name().as_str(), format!("failed to strip file: {}", file_path.display()).as_str())
			}
		    }
		    else if extension_str.contains("lib") {
			let command = format!("strip {} {}", lib_and_exec_args.join(" "), file_path.to_string_lossy());
			println!("{}", command);
			let status = Command::new("strip")
			    .args(&lib_and_exec_args)
			    .arg(&file_path)
			    .status().expect("Failed to strip file");
			if !status.success() {
			    die(get_repo_name().as_str(), format!("failed to strip file: {}", file_path.display()).as_str())
			}
		    }
		}
	    }
	    // Executable
	    else {
		// to detect if it is a elf executable
		let mut header = [0u8; 4];
		if let Err(_) = File::open(file_path.clone()).expect("Failed to open file").read_exact(&mut header) {
		    die(get_repo_name().as_str(), "Failed to read file header");
		}

		if header == [0x7f, 0x45, 0x4c, 0x46] {
		    // assume it is a executable
		    let command = format!("strip {} {}", lib_and_exec_args.join(" "), file_path.to_string_lossy());
		    println!("{}", command);
		    let status = Command::new("strip")
			.args(&lib_and_exec_args)
			.arg(&file_path)
			.status().expect("Failed to strip file");
		    if !status.success() {
			die(get_repo_name().as_str(), format!("failed to strip file: {}", file_path.display()).as_str())
		    }
		}
	    }
	}
    }
}

pub fn pkg_strip(pkg: &str) {
    // Strip package binaries and libraries. This saves space on the system as
    // well as on the tarballs we ship for installation.
    if Path::new(&*MAK_DIR).join(pkg).join("nostrip").exists() || *KISS_STRIP == "0" {
	return
    }

    log(pkg, "Stripping binaries and libraries");

    let manifest = format!("{}/{package_name}/{}/{package_name}/manifest", *PKG_DIR, PKG_DB, package_name = pkg);
    let files = read_a_files_lines(manifest.as_str()).expect("Failed to read manifest");

    for file in files {
	let real_file = format!("{}/{}/{}", *PKG_DIR, pkg, file).replace("//", "/");
	let real_file_path = Path::new(real_file.as_str());

	if real_file_path.is_dir() && is_matching_directory(real_file_path) {
	    strip_files_recursive(real_file_path);
	}
    }
}

// required for create_tar_archive function
pub fn add_dirs_to_tar_recursive<W: Write>(builder: &mut Builder<W>, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // avoid stack overflow issues
    // let mut stack: Vec<(PathBuf, PathBuf)> = vec![(PathBuf::from(dir), PathBuf::new())];

    // while let Some((path, rel_path)) = stack.pop() {
    // 	if path.is_dir() {
    // 	    let dir_name = path.file_name().ok_or_else(|| {
    // 		io::Error::new(io::ErrorKind::Other, format!("Failed to get directory name for path: {:?}", path))
    // 	    })?;

    // 	    let new_rel_path = rel_path.join(dir_name);

    // 	    builder.append_dir(new_rel_path.clone(), &path)?;

    // 	    let entries = fs::read_dir(&path)?;
    // 	    for entry in entries {
    // 		let entry = entry?;
    // 		let entry_path = entry.path();
    // 		stack.push((entry_path, new_rel_path.clone()));
    // 	    }
    // 	} else {
    // 	    let file_name = path.file_name().ok_or_else(|| {
    // 		io::Error::new(io::ErrorKind::Other, format!("Failed to get directory name for path: {:?}", path))
    // 	    })?;
    // 	    let new_rel_path = rel_path.join(file_name);
    // 	    builder.append_file(new_rel_path.clone(), &mut File::open(&path)?)?;
    // 	}
    // }

    for entry in fs::read_dir(dir)? {
	let entry = entry?;
	let entry_path = entry.path();
	// let file_type = entry.file_type();
	let rel_file_path = entry_path.strip_prefix(dir)?;

	if entry.path().is_dir() {
	    builder.append_dir_all(rel_file_path, entry.path())?;
	} else {
	    let mut file = File::open(entry.path())?;
	    builder.append_file(rel_file_path, &mut file)?;
	}
    }

    Ok(())
}

pub fn create_tar_archive(file: &str, compress_dir: &str, compress_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    let compress_path = Path::new(compress_dir);

    let file = match compress_type {
	"gz" | "xz" | "bz2" => File::create(file)?,
	_ => {
	    eprintln!("Unsupported compression type specified.");
	    return Ok(());
	}
    };

    match compress_type {
	"gz" => {
	    let gz_encoder = GzEncoder::new(file, flate2::Compression::default());
	    let mut gz_builder = Builder::new(gz_encoder);
	    add_dirs_to_tar_recursive(&mut gz_builder, compress_path)?;
	},
	"bz2" => {
	    let bz2_encoder = BzEncoder::new(file, bzip2::Compression::default());
	    let mut bz2_builder = Builder::new(bz2_encoder);
	    add_dirs_to_tar_recursive(&mut bz2_builder, compress_path)?;
	},
	"xz" => {
	    let xz_encoder = XzEncoder::new(file, 6);
	    let mut xz_builder = Builder::new(xz_encoder);
	    add_dirs_to_tar_recursive(&mut xz_builder, compress_path)?;
	},
	// does not work
	// "zst" => {
	//     let tar_zstd_file = File::create(file)?;
	//     let mut zstd_encoder = zstd::Encoder::new(tar_zstd_file, 0);
	//     let mut zstd_builder = Builder::new(&mut zstd_encoder);
	//     zstd_builder.append_dir_all(".", compress_path)?;
	//     zstd_encoder.finish()?;
	// }
	_ => {
	    eprintln!("Unsupported compression type");
	}
    }

    Ok(())
}

pub fn pkg_tar(pkg: &str) {
    log(pkg, "Creating tarball");

    let pkg_ver = pkg_find_version(pkg, false);
    let tar_file = format!("{}/{}@{}.tar.{}", *BIN_DIR, pkg, pkg_ver, *KISS_COMPRESS);

    let pkg_dir = format!("{}/{}/", *PKG_DIR, pkg);

    create_tar_archive(tar_file.as_str(), pkg_dir.as_str(), &*KISS_COMPRESS).expect("Failed to create tarball");

    log(pkg, "Successfully created tarball");
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
	return;
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
	pkg_strip(package);

	pkg_tar(package);
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
