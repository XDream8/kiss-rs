use checksum_lib::get_file_hash;
use kiss_manifest::{pkg_manifest, pkg_manifest_validate};
use source_lib::pkg_source_tar;

use shared_lib::globals::Config;

// logging
use shared_lib::signal::pkg_clean;
use shared_lib::{die, log};

use shared_lib::{
    is_symlink, mkcd, read_a_dir_and_sort, read_a_files_lines, remove_chars_after_last,
};

use std::fs::File;
use std::io::Read;

// for checking conflicts
use shared_lib::resolve_path;
use std::io::{BufRead, BufReader};

use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use install_lib::pkg_cache;

// threading
#[cfg(feature = "threading")]
use rayon::iter::ParallelIterator;
use shared_lib::{iter, sort_reverse};

// TODO: improve performance - it must be slow atm
fn pkg_conflicts(
    config: &Config,
    pkg: &str,
    manifest_file_path: &PathBuf,
    choice: bool,
    debug: bool,
) -> Result<(), std::io::Error> {
    if debug {
        log!(pkg, "Checking for package conflicts");
    }

    let mut resolved_paths: Vec<String> = Vec::new();
    let mut conflicts: Vec<String> = Vec::new();

    let manifest_contents: Vec<String> = read_a_files_lines(manifest_file_path)?;
    for line in manifest_contents {
        // store absolute paths in vector
        if line.ends_with('/') {
            continue;
        }
        if let Some(resolved_path) = resolve_path(config, line.as_str()) {
            resolved_paths.push(format!("{}", resolved_path.to_string_lossy()));
        }
    }

    // only get manifest files
    let sys_manifest_files: Vec<PathBuf> = iter!(read_a_dir_and_sort(
        config.sys_db.to_string_lossy().to_string(),
        true,
        &[]
    ))
    .filter(|file| file.file_name().unwrap().to_str() == Some("manifest"))
    .map(|name| name.to_path_buf())
    .collect();

    let mut conflicts_found: bool = false;
    let mut safe: bool = false;

    for sys_manifest_path in sys_manifest_files {
        // do not check against the package manifest, if package wanted to be installed is already installed
        if sys_manifest_path.to_string_lossy()
            == format!("{}/{}", config.sys_db.to_string_lossy(), pkg)
        {
            continue;
        };

        let sys_manifest_file: File = fs::File::open(sys_manifest_path).unwrap();
        let sys_manifest_reader: BufReader<File> = BufReader::new(sys_manifest_file);

        for line in sys_manifest_reader.lines().flatten() {
            let found: bool = iter!(resolved_paths).any(|path| path == &line);
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

    if choice && safe && conflicts_found {
        // Handle conflicts and create choices
        let choice_directory: PathBuf = config.tar_dir.join(pkg).join(&config.cho_db);
        // Create the "choices" directory inside of the tarball.
        // This directory will store the conflicting file.
        fs::create_dir_all(&choice_directory)?;

        let mut choices_created: usize = 0;

        for conflict in conflicts {
            println!("Found conflict: {}", conflict);

            let new_file_name: String = format!(
                "{}>{}",
                pkg,
                conflict.trim_start_matches('/').replace('/', ">")
            );
            let choice_file_path: PathBuf = choice_directory.join(new_file_name);
            let real_conflict_path: PathBuf = config.tar_dir.join(pkg).join(conflict);

            fs::rename(real_conflict_path, choice_file_path)?;
            choices_created += 1;
        }

        if choices_created > 0 {
            log!(pkg, "Converted all conflicts to choices (kiss a)");
            // Rewrite the package's manifest to update its location
            // to its new spot (and name) in the choices directory.
            pkg_manifest(config, pkg, &config.tar_dir);
        }
    } else if conflicts_found {
        println!("Package '{}' conflicts with another package !>", pkg);
        println!("Run 'KISS_CHOICE=1 kiss i '{}' to add conflicts !>", pkg);
        die!("", "as alternatives. !>");
    }

    Ok(())
}

fn pkg_installable(config: &Config, pkg: &str, depends_file_path: String, debug: bool) {
    if debug {
        log!(pkg, "Checking if package installable");
    }

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

        if config.sys_db.join(dep.clone()).exists() {
            continue;
        }

        println!("{} {}", dep, dependency_type);

        count += 1;
    }

    if count != 0 {
        die!(pkg, "Package not installable, missing {} package(s)", count);
    }
}

fn pkg_etc<'a>(
    sum_old: Option<&'a str>,
    sum_sys: Option<&'a str>,
    tar_dir: &str,
    file: &str,
) -> i32 {
    let sum_new: String = match get_file_hash((tar_dir.to_owned() + file).as_str()) {
        Ok(hash) => hash,
        _ => String::new(),
    };

    if let Some(sum_old) = sum_old {
        if let Some(_sum_sys) = sum_sys {
            // old = Y, sys = X, new = Y
            if sum_old == sum_new && !sum_old.is_empty() {
                return 1;
            }
        }
    } else if let Some(_sum_sys) = sum_sys {
        // old = X, sys = X, new = X
        // old = X, sys = Y, new = Y
        // old = X, sys = X, new = Y
        return 0;
    }

    // All other cases
    println!("Saving {} as {}.new", file, file);
    2
}

fn file_rwx(file_path: &Path) -> Result<u32, std::io::Error> {
    let permissions = fs::metadata(file_path)?.permissions();

    let oct: u32 = permissions.mode();

    Ok(oct)
}

fn pkg_install_files(
    config: &Config,
    files: &Vec<String>,
    pkg_root: &Path,
    source_dir: &Path,
    overwrite: bool,
    verify: bool,
) -> Result<(), std::io::Error> {
    for file in files {
        let file_stripped: &str = file.strip_prefix('/').unwrap_or(file);
        let mut dest_path: PathBuf = pkg_root.join(file_stripped);
        let source_path: PathBuf = source_dir.join(file_stripped);

        if verify && dest_path.exists() {
            continue;
        }

        let dest_parent: &Path = dest_path.parent().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid file path (no parent directory)",
        ))?;

        // create parent directory of destination if it does not exist
        if !dest_parent.exists() {
            fs::create_dir_all(dest_parent)?;
        }

        // if a directory does not exist then create it with proper permissions
        // then continue with the next file
        if source_path.is_dir() && !dest_path.exists() {
            // Get octal permissions using file_rwx function.
            let octal_permissions: u32 = file_rwx(&source_path)?;
            // create directory
            fs::create_dir_all(&dest_path)?;
            // Set permissions for the directory.
            let permissions = fs::Permissions::from_mode(octal_permissions);
            fs::set_permissions(&dest_path, permissions)?;
            continue;
        } else if source_path.is_dir() {
            continue;
        }

        if let Some(parent) = source_path.parent() {
            if !parent.exists() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Source file path does not exist",
                ));
            }
        }

        // symlink checks
        if dest_path.is_symlink() || is_symlink(&dest_path) {
            if overwrite {
                // ignore errors
                let _ = fs::remove_file(&dest_path);
            } else {
                continue;
            }
        }

        // verify
        if verify && dest_path.exists() {
            continue;
        } else if overwrite && dest_path.exists() && dest_path.is_file() {
            // ignore errors
            let _ = fs::remove_file(&dest_path);
        }

        // /etc file checks
        if file.contains("/etc/") {
            let sum_old: Option<String> = match verify {
                true => {
                    let mut buffer: String = String::new();
                    let mut old_file: File = fs::File::open(&dest_path)?;
                    old_file.read_to_string(&mut buffer)?;
                    Some(buffer.trim().to_owned())
                }
                false => None,
            };
            let sum_sys: Option<String> = match get_file_hash(file) {
                Ok(hash) => Some(hash),
                _ => None,
            };
            match pkg_etc(
                sum_old.as_deref(),
                sum_sys.as_deref(),
                config.tar_dir.to_str().unwrap_or(""),
                file,
            ) {
                3 => dest_path.set_extension("new"),
                _ => continue,
            };
        }

        // install
        if source_path.is_symlink() || is_symlink(source_path.as_path()) {
            fs::copy(&source_path, &dest_path)?;
        } else {
            let temp_dest_path: PathBuf = create_temp_dest_path(&dest_path)?;
            fs::copy(&source_path, &temp_dest_path)?;
            fs::rename(&temp_dest_path, &dest_path)?;
        }

        if config.debug {
            if overwrite {
                println!("Installing: {}", file);
            } else {
                println!("Installing(in verify mode): {}", file);
            }
        }
    }

    Ok(())
}

// used by pkg_install_files
fn create_temp_dest_path(dest_path: &Path) -> Result<PathBuf, std::io::Error> {
    let file_name: &OsStr = dest_path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file path")
    })?;

    let temp_file_name: String = format!("__kiss-tmp-{}", file_name.to_string_lossy());
    let mut temp_dest_path: PathBuf = dest_path.to_path_buf();
    temp_dest_path.set_file_name(temp_file_name);

    Ok(temp_dest_path)
}

fn pkg_remove_files(
    kiss_root: &Path,
    files: &Vec<String>,
    debug: bool,
) -> Result<(), std::io::Error> {
    let mut broken_symlinks: Vec<PathBuf> = Vec::new();

    for file in files {
        if file.contains("/etc") {
            let mut sum_pkg: String = String::new();
            fs::File::open(kiss_root.join(file))?.read_to_string(&mut sum_pkg)?;

            let hash: String = get_file_hash(
                format!("{}/{}", kiss_root.to_string_lossy(), file)
                    .replace("//", "/")
                    .as_str(),
            )?;

            if hash != sum_pkg {
                println!("Skipping {} (modified)", file);
                continue;
            }
        }

        let relative_file_path: &Path = Path::new(file.as_str())
            .strip_prefix("/")
            .unwrap_or(Path::new(file.as_str()));
        let full_path: PathBuf = kiss_root.join(relative_file_path);

        if let Ok(metadata) = fs::metadata(&full_path) {
            if metadata.is_dir() {
                // ignore errors when removing directories
                // this is needed because we cant just remove everything in /var/db/kiss/, /var/db/ or /usr/
                if fs::remove_dir(&full_path).is_err() {};
            } else {
                fs::remove_file(&full_path)?;
            }
        }

        if let Ok(target) = fs::read_link(&full_path) {
            if !target.exists() {
                broken_symlinks.push(full_path);
            }
        }

        if debug {
            println!("Removing: {}", file);
        }
    }

    // Remove all broken directory symlinks.
    for symlink in broken_symlinks {
        if let Ok(target) = fs::read_link(&symlink) {
            if !target.exists() {
                fs::remove_file(&symlink)?;
            }
        }
    }

    Ok(())
}

pub fn pkg_install(config: &Config, package_tar: &str) -> Result<(), std::io::Error> {
    // create pkg and tar_file variable but dont set a value
    let pkg: String;
    // pkg tarball to be used
    let tar_file: String;

    if package_tar.contains(".tar.") {
        tar_file = package_tar.to_owned();
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
        if let Some(tarball) = pkg_cache(config, package_tar) {
            tar_file = tarball;
        } else {
            die!(package_tar, "Not yet built");
        }

        pkg = package_tar.to_owned();
    }

    // cd into extract directory
    let extract_dir: PathBuf = config.tar_dir.join(pkg.as_str());
    mkcd(extract_dir.to_str().unwrap_or(""));

    // extract to current dir
    pkg_source_tar(tar_file.clone(), false);

    let manifest_path: PathBuf = extract_dir
        .join(&config.pkg_db)
        .join(pkg.as_str())
        .join("manifest");

    if !manifest_path.exists() {
        println!(
            "{}, {}",
            extract_dir.to_string_lossy(),
            manifest_path.to_string_lossy()
        );
        die!("Not a valid KISS package");
    }

    if !config.force {
        pkg_manifest_validate(
            pkg.as_str(),
            extract_dir.to_str().unwrap_or(""),
            manifest_path.clone(),
            config.debug,
        );
        pkg_installable(
            config,
            pkg.as_str(),
            format!("./{}/{}/depends", config.pkg_db, pkg),
            config.debug,
        );
    }

    pkg_conflicts(
        config,
        pkg.as_str(),
        &manifest_path,
        config.choice,
        config.debug,
    )?;

    log!(
        "Installing",
        pkg.to_owned() + ":",
        tar_file
            .split('/')
            .last()
            .expect("Failed to get tar_file name")
    );

    //
    let tar_man: String = format!("{}/{}/manifest", config.pkg_db, pkg);

    let old_files: Vec<String> = read_a_files_lines(&tar_man)?;
    let new_files: Vec<String> = read_a_files_lines(manifest_path.clone())?;

    // Generate a list of files which exist in the currently installed manifest
    // but not in the newer (to be installed) manifest.
    let manifest_diff: Vec<String> = old_files
        .into_iter()
        .filter(|f| !new_files.contains(f))
        .collect();

    // let sorted_files: Vec<String> = {
    //     let mut files: Vec<String> = read_a_files_lines(&tar_man)?;
    //     files.sort_unstable();
    //     files
    // };

    // Reverse the manifest file so that we start shallow and go deeper as we
    // iterate over each item. This is needed so that directories are created
    // going down the tree.
    let manifest_reverse: Vec<String> = {
        let mut files: Vec<String> = read_a_files_lines(tar_man)?;
        // sort manifest reverse alphabetically and then reverse
        sort_reverse!(files);
        files.reverse();
        files
    };

    let install_files_result = pkg_install_files(
        config,
        &manifest_reverse,
        Path::new(&config.kiss_root),
        &extract_dir,
        true,
        false,
    );
    let remove_files_result =
        pkg_remove_files(Path::new(&config.kiss_root), &manifest_diff, config.debug);
    let install_files_result2 = pkg_install_files(
        config,
        &manifest_reverse,
        Path::new(&config.kiss_root),
        &extract_dir,
        false,
        true,
    );

    // handle all errors gracefully
    match (
        install_files_result,
        remove_files_result,
        install_files_result2,
    ) {
        (Ok(_), Ok(_), Ok(_)) => log!("Installed successfully:", pkg),
        (Err(err), _, _) => log_and_notify_error("Error installing files:", pkg, err),
        (_, Err(err), _) => log_and_notify_error("Error removing files:", pkg, err),
        (_, _, Err(err)) => log_and_notify_error("Error verifying files:", pkg, err),
    }

    Ok(())
}

fn log_and_notify_error(log: &str, pkg: String, err: impl std::error::Error) {
    log!(log, err);
    die!(
        "Error installing:",
        pkg.to_owned() + ":",
        "Filesystem now dirty, manual repair needed."
    );
}
