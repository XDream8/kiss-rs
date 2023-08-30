use checksum_lib::{get_file_hash, pkg_verify};
use kiss_manifest::pkg_manifest;
use search_lib::{pkg_cache, pkg_find_path};
use source_lib::{pkg_source, pkg_source_resolve, pkg_source_tar, pkg_tar};

use shared_lib::{
    copy_folder, get_current_working_dir, get_directory_name, mkcd, read_a_files_lines,
    remove_chars_after_last,
};
use shared_lib::{
    pkg_get_provides, prompt, read_sources, run_action, run_command, set_env_variable_if_undefined,
};

// manage global variables
use shared_lib::globals::{Config, Dependencies};

// logging
use shared_lib::signal::pkg_clean;
use shared_lib::{die, log};

// std
use std::fs::{self, File};
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
// build
use std::process::{Child, Command, ExitStatus, Stdio};

pub fn pkg_extract(config: &Config, pkg: &str, repo_dir: &String) {
    if config.debug || config.verbose {
        log!(pkg.to_owned() + ":", "Extracting sources");
    }

    let sources_file: String = format!("{}/sources", repo_dir);

    let sources: Vec<(String, String)> =
        read_sources(sources_file.as_str()).expect("Failed to read sources file");

    for (source, dest) in sources.iter() {
        let (res, des): (String, String) =
            pkg_source_resolve(config, pkg, repo_dir, source, dest, false);

        // temporary solution - need to find a better way
        let dest_path: PathBuf = config.mak_dir.join(pkg);
        // Create the source's directories if not null.
        if res != des {
            mkcd(dest_path.to_string_lossy().to_string());
        }

        if res.contains("git+") {
            let dest_path = dest_path.join(dest);
            copy_folder(Path::new(des.as_str()), dest_path.as_path())
                .expect("Failed to copy git source");
        } else if des.contains(".tar.") {
            pkg_source_tar(res, &dest_path, true);
        } else {
            let file_name = Path::new(res.as_str()).file_name().unwrap();
            let dest_path: PathBuf = dest_path.join(file_name);
            // println!("{dest_path:?}");
            fs::copy(res.clone(), &dest_path).expect("Failed to copy file");
        }
    }
}

// required for stripping
fn is_matching_directory(path: &Path) -> bool {
    let file_name = path.file_name().unwrap_or_default();
    let parent_dir_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(std::ffi::OsStr::to_str);

    let is_sbin: bool = file_name == "sbin";
    let is_bin: bool = file_name == "bin";
    let is_lib: bool = parent_dir_name == Some("lib");

    is_sbin || is_bin || is_lib
}

// for stripping
fn strip_files_recursive(repo_name: &str, directory: &Path) {
    let entries = fs::read_dir(directory).expect("Failed to read directory");

    let lib_and_exec_args: Vec<&str> = vec!["-s", "-R", ".comment", "-R", ".note"];
    let object_and_static_lib_args: Vec<&str> = vec!["-g", "-R", ".comment", "-R", ".note"];

    for entry in entries {
        let entry = entry.unwrap();
        let file_path: PathBuf = entry.path();
        let file_path_string: String = file_path.to_string_lossy().to_string();

        if file_path.is_dir() {
            strip_files_recursive(repo_name, &file_path);
        } else if file_path.is_file() {
            if let Some(extension) = file_path.extension() {
                if let Some(extension_str) = extension.to_str() {
                    if extension_str == "o" || extension_str == "a" {
                        let mut args: Vec<&str> = object_and_static_lib_args.clone();
                        args.push(&file_path_string);
                        if let Err(err) = run_command("strip", &args) {
                            die!("Failed to strip file:", file_path_string, "-", err);
                        }
                    } else if extension_str.contains("lib") {
                        // assume its a library
                        let mut args: Vec<&str> = lib_and_exec_args.clone();
                        args.push(&file_path_string);
                        if let Err(err) = run_command("strip", &args) {
                            die!("Failed to strip file:", file_path_string, "-", err);
                        }
                    }
                }
            }
            // Executable
            else {
                // to detect if it is a elf executable
                let mut header = [0u8; 4];
                if File::open(file_path.clone())
                    .expect("Failed to open file")
                    .read_exact(&mut header)
                    .is_err()
                {
                    die!(repo_name, "Failed to read file header");
                }

                if header == [0x7f, 0x45, 0x4c, 0x46] {
                    // assume it is a executable
                    let mut args: Vec<&str> = lib_and_exec_args.clone();
                    args.push(&file_path_string);
                    if let Err(err) = run_command("strip", &args) {
                        die!("Failed to strip file:", file_path_string, "-", err);
                    }
                }
            }
        }
    }
}

fn pkg_strip(config: &Config, pkg: &str) {
    // Strip package binaries and libraries. This saves space on the system as
    // well as on the tarballs we ship for installation.
    if config.mak_dir.join(pkg).join("nostrip").exists() || !config.strip {
        return;
    }

    log!(pkg, "Stripping binaries and libraries");

    let manifest = format!(
        "{}/{package_name}/{}/{package_name}/manifest",
        config.pkg_dir.to_string_lossy(),
        config.pkg_db,
        package_name = pkg
    );
    let files = read_a_files_lines(manifest.as_str()).expect("Failed to read manifest");

    for file in files {
        let real_file =
            format!("{}/{}/{}", config.pkg_dir.to_string_lossy(), pkg, file).replace("//", "/");
        let real_file_path = Path::new(real_file.as_str());

        if real_file_path.is_dir() && is_matching_directory(real_file_path) {
            strip_files_recursive(pkg, real_file_path);
        }
    }
}

fn pkg_etcsums(config: &Config, pkg: &str) {
    // Generate checksums for each configuration file in the package's /etc/
    // directory for use in "smart" handling of these files.
    if config.debug {
        log!(pkg, "Generating etcsums");
    }

    // Minor optimization - skip packages without /etc/.
    if !config.pkg_dir.join(pkg).join("etc").is_dir() {
        return;
    }

    let pkg_db_path: String = format!(
        "{}/{package_name}/{}/{package_name}",
        config.pkg_dir.to_string_lossy(),
        config.pkg_db,
        package_name = pkg
    );

    let manifest: String = format!("{}/manifest", pkg_db_path);
    let manifest_file: File = File::open(manifest).expect("Failed to open manifest file");
    let manifest_reader = io::BufReader::new(manifest_file);

    // store etc files in this vector
    let mut etc_files: Vec<String> = Vec::new();

    for line in manifest_reader.lines() {
        let etc: String = line.unwrap();

        if etc.starts_with("/etc") && !etc.ends_with('/') {
            if let Some(etc_file) = etc.strip_prefix('/') {
                let etc: String =
                    format!("{}/{}/{}", config.pkg_dir.to_string_lossy(), pkg, etc_file);
                let etc_path = Path::new(&etc);

                // Check if the path is a symbolic link
                if etc_path
                    .symlink_metadata()
                    .unwrap()
                    .file_type()
                    .is_symlink()
                {
                    etc_files.push(String::from("/dev/null"));
                } else {
                    etc_files.push(etc);
                }
            }
        }
    }

    let etcsums_path: String = format!("{}/etcsums", pkg_db_path);
    let mut etcsums_file = File::create(etcsums_path).expect("Failed to create etcsums file");

    for etc_file in etc_files {
        let hash = get_file_hash(etc_file.as_str()).expect("Failed to get file hash");
        etcsums_file
            .write_all(hash.as_bytes())
            .expect("Failed to write hash to etcsums file");
    }
}

// the method we use to store deps and explicit deps is different from original kiss pm.
// we only store implicit deps in DEPS global var and explicit deps in EXPLICIT global var
#[inline(always)]
fn pkg_depends(
    config: &Config,
    dependencies: &mut Dependencies,
    pkg: &String,
    expl: bool,
    filter: bool,
    dep_type: Option<&str>,
) {
    // check if user defined a replacement
    let pkg: &String = &pkg_get_provides(pkg, &config.provides_db).unwrap_or(pkg.to_owned());
    // since pkg_find function sets REPO_DIR and REPO_NAME, run it first
    let repo_dir: PathBuf = pkg_find_path(config, pkg, None).unwrap_or(PathBuf::new());

    // Resolve all dependencies and generate an ordered list. The deepest
    // dependencies are listed first and then the parents in reverse order.
    if dependencies.normal.contains(pkg) {
        return;
    }

    if !filter || dependencies.explicit.contains(pkg) || !expl && config.sys_db.join(pkg).exists() {
        return;
    }

    if !repo_dir.exists() || repo_dir.join("depends").exists() {
        let depends: Vec<String> = read_a_files_lines(repo_dir.join("depends")).unwrap();
        for dependency in depends {
            if dependency.starts_with('#') {
                continue;
            }

            let (dep, dependency_type): (String, Option<&str>) = if dependency.contains(" make") {
                let binding = &remove_chars_after_last(&dependency, ' ').trim_end();
                (binding.to_string(), Some("make"))
            } else {
                (dependency, None)
            };

            pkg_depends(config, dependencies, &dep, false, filter, dependency_type);
        }
    } else {
        return;
    }

    // add to dependency vec
    if !expl || dep_type.unwrap_or("") == "make" && pkg_cache(config, pkg).is_none() {
        dependencies.normal.push(pkg.to_owned());
    }
}

pub fn pkg_build_all<T>(config: &Config, dependencies: &mut Dependencies, packages: Vec<T>)
where
    T: AsRef<str> + std::clone::Clone + std::fmt::Display,
{
    // find dependencies
    if !packages.is_empty() {
        for package in packages {
            pkg_depends(config, dependencies, &package.to_string(), true, true, None);
            dependencies.explicit.push(package.to_string());
        }
    } else {
        let current_dir: String = get_current_working_dir();
        let package: &str = get_directory_name(&current_dir);
        pkg_depends(config, dependencies, &package.to_owned(), true, true, None);
        dependencies.explicit.push(package.to_owned());
    }

    // If an explicit package is a dependency of another explicit package,
    // remove it from the explicit list.
    for package in dependencies.explicit.clone() {
        if dependencies.normal.contains(&package) {
            dependencies.explicit.retain(|x| x != &package)
        }
    }

    // log
    if dependencies.normal.is_empty() {
        println!("Building: {}", dependencies.explicit.join(" "))
    } else {
        println!(
            "Building: explicit: {}, implicit: {}",
            dependencies.explicit.join(" ").trim(),
            dependencies.normal.join(" ")
        )
    }

    // prompt
    if !dependencies.normal.is_empty() && config.prompt {
        prompt(None);
    }

    if config.debug || config.verbose {
        println!("Checking for pre-built dependencies");
    }
    // Install any pre-built dependencies if they exist in the binary
    // directory and are up to date.
    for pkg in dependencies.normal.clone() {
        if pkg_cache(config, &pkg).is_some() {
            log!(pkg.to_owned() + ":", "Found pre-built binary");
            dependencies.normal.retain(|x| x != &pkg);
            if let Err(err) = run_action("install", Some(&[&pkg])) {
                die!("Failed to install package:", pkg, err);
            }
        }
    }

    let all_packages: Vec<&String> = dependencies
        .normal
        .iter()
        .chain(dependencies.explicit.iter())
        .collect();

    // download and check sources
    for package in &all_packages {
        pkg_source(config, package, false, true);
        let repo_dir = pkg_find_path(config, package, None)
            .unwrap_or_else(|| die!(package.to_string() + ":", "Failed to get version"))
            .to_string_lossy()
            .to_string();

        if Path::new(&repo_dir).join("sources").exists() {
            pkg_verify(config, package, repo_dir);
        }
    }

    // build process
    let mut build_cur: usize = 0;
    let package_count: usize = all_packages.len();

    for package in all_packages {
        // print status
        build_cur += 1;
        let build_status: String = format!("Building package ({}/{})", build_cur, package_count);
        log!(package.to_owned() + ":", build_status);

        let repo_dir: String = pkg_find_path(config, package, None)
            .unwrap_or_else(|| die!(package.to_owned() + ":", "Failed to get version"))
            .to_string_lossy()
            .to_string();

        if Path::new(repo_dir.as_str()).join("sources").exists() {
            pkg_extract(config, package, &repo_dir);
        }

        pkg_build(config, package, repo_dir);
        pkg_manifest(config, package, &config.pkg_dir);
        pkg_strip(config, package);

        pkg_etcsums(config, package);
        pkg_tar(config, package);

        if !dependencies.explicit.contains(package) {
            log!(
                format!("{}:", package),
                "Needed as a dependency or has an update, installing"
            );
            // pkg_install(pkg, true).expect("Failed to install package");
            //     run_action_as_root(vec!["install", package], true);
            if let Err(err) = run_action("install", Some(&[package])) {
                die!("Failed to install package:", package, err);
            }
        }
    }

    if config.prompt {
        // let mut action: Vec<&str> = vec!["install"];
        // action.extend(explicit.iter().map(|s| s.as_str()));
        prompt(Some(format!(
            "Install built packages? [{}]",
            dependencies.explicit.join(" ")
        )));
        if let Err(err) = run_action("install", None) {
            die!("Failed to install: {}", err);
        }
    }
}

fn pkg_build(config: &Config, pkg: &str, repo_dir: String) {
    mkcd(format!("{}/{}", config.mak_dir.to_string_lossy(), pkg).as_str());

    log!(pkg.to_owned() + ":", "Starting build");

    set_env_variable_if_undefined("AR", "ar");
    set_env_variable_if_undefined("CC", "cc");
    set_env_variable_if_undefined("CXX", "c++");
    set_env_variable_if_undefined("NM", "nm");
    set_env_variable_if_undefined("RANLIB", "ranlib");

    let executable: String = format!("{}/build", repo_dir);
    let install_dir: PathBuf = config.pkg_dir.join(pkg);

    let mut child: Child = Command::new(executable)
        .arg(install_dir.to_string_lossy().to_string())
        .stdout(if config.quiet {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .spawn()
        .expect("Failed to execute build file");
    // wait for build to finish
    let status: ExitStatus = child.wait().expect("Failed to wait for command");
    if status.success() {
        // Copy the repository files to the package directory.
        let pkg_db_dir: String = format!(
            "{}/{package_name}/{}/{package_name}",
            config.pkg_dir.to_string_lossy(),
            config.pkg_db,
            package_name = pkg
        );

        mkcd(pkg_db_dir.as_str());

        if let Err(err) = copy_folder(Path::new(repo_dir.as_str()), Path::new(pkg_db_dir.as_str()))
        {
            die!("Failed to copy repository files:", err)
        }

        // give info
        log!(pkg, "Successfully built package")
    } else {
        die!(pkg, "Build failed")
    }
}
