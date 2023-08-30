pub mod cli;
pub mod logging;
pub mod signal;
pub mod threading;

// re-exports
pub use crate::cli::flags;
pub use crate::cli::globals;

use crate::globals::Config;
use crate::signal::pkg_clean;

use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader, Result};
use std::path::{Path, PathBuf};

use seahorse::Context;
use std::env;

// for function run_action_as_root
use libc::getuid;
use std::os::unix::fs::MetadataExt;
// use std::process::{self, Child, Command, ExitStatus, Stdio};
use std::process::{Command, ExitStatus};

#[cfg(feature = "threading")]
use rayon::iter::ParallelIterator;

// Functions

#[inline]
pub fn pkg_get_provides(pkg: &str, provides_path: &Path) -> Result<String> {
    let file: File = File::open(provides_path)?;
    let reader: BufReader<File> = BufReader::new(file);

    // find the replacement if there is any
    for line in reader.lines() {
        let line: &String = &line?;
        if line.starts_with('#') {
            continue;
        };
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() == 2 && parts[1] == pkg {
            return Ok(parts[0].to_owned());
        }
    }

    // if we did not find an replacement return pkg
    Ok(pkg.to_owned())
}

#[inline]
pub fn prompt(log_message: Option<String>) {
    if let Some(log_message) = log_message {
        log!(log_message);
    }

    // Ask for confirmation if extra packages need to be built.
    println!("Continue?: Press Enter to continue or Ctrl+C to abort");

    // get user input
    io::stdin().lock().lines().next();
}

#[inline]
pub fn get_args(c: &Context) -> Vec<&str> {
    iter!(c.args).map(|arg| arg.as_str()).collect()
}

pub fn run_command(command: &str, args: &Vec<&str>) -> Result<ExitStatus> {
    let full_command = format!("{} {}", command, args.join(" "));
    println!("{}", full_command);
    let status: ExitStatus = Command::new(command).args(args).status()?;
    Ok(status)
}

// file operations
pub fn cat(path: &Path) -> Result<String> {
    let file_bytes: Vec<u8> = fs::read(path)?;
    let buffer: String = String::from_utf8(file_bytes).unwrap_or(String::new());

    Ok(buffer)
}

#[inline]
pub fn read_a_files_lines(
    file_path: impl AsRef<Path> + AsRef<std::ffi::OsStr>,
) -> Result<Vec<String>> {
    if Path::new(&file_path).exists() {
        let f: File = File::open(file_path)?;
        let buf: BufReader<File> = BufReader::new(f);
        let lines: Vec<String> = buf.lines().map_while(Result::ok).collect();

        Ok(lines)
    } else {
        Ok(vec![])
    }
}

#[inline]
pub fn mkcd(folder_name: impl AsRef<Path> + AsRef<std::ffi::OsStr> + AsRef<str>) {
    if let Err(err) = fs::create_dir_all(&folder_name) {
        die!("Failed to create folder:", err);
    }
    if let Err(err) = env::set_current_dir(&folder_name) {
        die!("Failed to change directory:", err);
    }
}

pub fn remove_chars_after_last(input: &str, ch: char) -> &str {
    if let Some(index) = input.rfind(ch) {
        &input[..index]
    } else {
        input
    }
}

pub fn get_current_working_dir() -> String {
    match env::current_dir() {
        Ok(current_dir) => current_dir.to_string_lossy().into_owned(),
        Err(_) => String::from(""),
    }
}

pub fn get_directory_name(path: &str) -> &str {
    let path: &Path = Path::new(path);
    match path.file_name() {
        Some(folder_name) => match folder_name.to_str() {
            Some(name) => name,
            None => "",
        },
        None => "",
    }
}

#[inline]
pub fn get_env_variable(env: &str, default_value: String) -> String {
    // get output of environment variable
    env::var(env).unwrap_or(default_value)
}

pub fn set_env_variable_if_undefined(name: &str, value: &str) {
    if env::var(name).is_err() {
        env::set_var(name, value);
    }
}

// used by build command
pub fn copy_folder(source: &Path, destination: &Path) -> Result<()> {
    if source.is_dir() {
        fs::create_dir_all(destination)?;

        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let source_path = entry.path();
            let destination_path = destination.join(entry.file_name());

            if source_path.is_dir() {
                copy_folder(&source_path, &destination_path)?;
            } else {
                fs::copy(&source_path, &destination_path)?;
            }
        }
    }

    Ok(())
}

#[inline]
pub fn read_a_dir_and_sort(
    path: impl AsRef<Path> + AsRef<std::ffi::OsStr> + AsRef<str>,
    recursive: bool,
    skip_ext: &[&str],
) -> Vec<PathBuf> {
    let mut filtered_entries: Vec<PathBuf> = Vec::new();

    let folder_path: &Path = Path::new(&path);

    if folder_path.is_dir() {
        let entries: Vec<_> = match fs::read_dir(folder_path) {
            Ok(entries) => entries.collect(),
            Err(e) => die!("Failed to read directory:", e),
        };

        // Parallelize the directory traversal and filtering
        let filtered_entries_par: Vec<PathBuf> = iter!(entries)
            .filter_map(|entry| {
                let entry = entry.as_ref().unwrap();
                let path = entry.path();

                if !skip_ext.is_empty() {
                    if let Some(file_name) = path.file_name() {
                        let file_name = file_name.to_string_lossy().into_owned();
                        let skip_file = skip_ext.iter().any(|ext| file_name.ends_with(ext));
                        if skip_file {
                            return None;
                        }
                    }
                }

                Some(path)
            })
            .collect();

        filtered_entries.extend(filtered_entries_par.clone());

        if recursive {
            // Parallelize the recursive traversal and filtering
            let subfolder_entries: Vec<Vec<PathBuf>> = iter!(filtered_entries_par)
                .filter_map(|path| {
                    if path.is_dir() {
                        Some(read_a_dir_and_sort(
                            &*path.to_string_lossy(),
                            recursive,
                            skip_ext,
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            for entries in subfolder_entries {
                filtered_entries.extend(entries);
            }
        }
    }

    // Parallelize the sorting task
    sort!(filtered_entries);

    filtered_entries
}

#[inline]
pub fn tmp_file(config: &Config, name: &str, suffix: &str) -> Result<(File, PathBuf)> {
    let tmp_file_name: String = format!("{}-{}", name, suffix);
    let tmp_file_path: PathBuf = config.tmp_dir.join(tmp_file_name);
    let tmp_file: File = File::create(&tmp_file_path)?;

    Ok((tmp_file, tmp_file_path))
}

#[inline]
pub fn read_sources(
    path: impl AsRef<Path> + AsRef<std::ffi::OsStr> + AsRef<str>,
) -> Result<Vec<(String, String)>> {
    let sources: Vec<String> = read_a_files_lines(path)?;

    // filter out items that starts with ’#’, then return (source, dest)
    let result: Vec<(String, String)> = iter!(sources)
        .filter(|&x| !x.starts_with('#'))
        .map(|source| {
            // consider user-given folder name
            if source.contains(' ') {
                let source_parts: Vec<String> = source.split(' ').map(|l| l.to_owned()).collect();
                (
                    source_parts.first().unwrap().to_owned(),
                    source_parts
                        .last()
                        .unwrap()
                        .to_owned()
                        .trim_end_matches('/')
                        .to_owned(),
                )
            } else if !source.contains("git+") && !source.contains("http") {
                (source.to_string(), source.to_string())
            } else {
                (source.to_string(), String::new())
            }
        })
        .collect();

    Ok(result)
}

#[inline]
pub fn resolve_path(config: &Config, path: &str) -> Option<PathBuf> {
    let rpath: PathBuf = config.kiss_root.join(path.trim_start_matches('/'));

    let parent: &Path = rpath.parent()?;

    let absolute_path: PathBuf = if parent.is_absolute() {
        parent
            .to_path_buf()
            .join(rpath.file_name().unwrap_or_default())
    } else {
        config
            .kiss_root
            .join(parent)
            .join(rpath.file_name().unwrap_or_default())
    };

    Some(absolute_path)
}

#[inline]
pub fn is_symlink(path: &Path) -> bool {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        metadata.file_type().is_symlink()
    } else {
        false
    }
}

pub fn am_owner(file_or_dir: &str) -> Result<bool> {
    let metadata = fs::metadata(file_or_dir)?;
    let current_uid = unsafe { getuid() };
    Ok(metadata.uid() == current_uid)
}

// used by kiss-build to install deps and packages
pub fn run_action(binary_name: &str, binary_args: Option<&[&str]>) -> Result<()> {
    // Collect command line arguments
    let args: Vec<String> = env::args().collect();

    // Get the path to the current executable
    let exe_path: PathBuf = env::current_exe()?;

    // Get the directory containing the executable
    let exe_dir: &Path = exe_path.parent().ok_or(io::Error::new(
        io::ErrorKind::Other,
        "Failed to get the directory",
    ))?;

    // Construct the binary path in the same directory as the wrapper
    let binary_path_in_exe_dir: PathBuf = exe_dir.join(format!("kiss-{}", binary_name));

    let binary_path: Option<PathBuf> = if binary_path_in_exe_dir.exists() {
        Some(binary_path_in_exe_dir)
    } else {
        // If the binary is not found in the same directory as the wrapper, search the system PATH
        env::var("PATH")
            .expect("Failed to get PATH environment variable")
            .split(':')
            .find_map(|path| {
                let path: String = path.to_string();
                let binary_name: String = format!("kiss-{}", binary_name);
                let binary_path: PathBuf = Path::new(&path).join(&binary_name);
                if binary_path.exists() {
                    Some(binary_path)
                } else if Path::new(&path).exists() {
                    let entries: Vec<_> = fs::read_dir(path).ok()?.collect();

                    entries.iter().find_map(|entry| {
                        let entry = entry.as_ref().unwrap();
                        let path: PathBuf = entry.path();

                        if let Some(file_name) = path.file_name() {
                            let file_name: String = file_name.to_string_lossy().into_owned();

                            if file_name.starts_with(&binary_name) {
                                Some(path)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            })
    };

    // execute command
    if let Some(binary_path) = binary_path {
        // Build the command to execute the binary
        let mut command: Command = Command::new(binary_path);
        // Pass all arguments except the first two (wrapper and binary name)
        command.args(&args[2..]);
        if let Some(binary_args) = binary_args {
            command.args(binary_args);
        }

        // Execute the binary
        command.status()?;
    }

    Ok(())
}
