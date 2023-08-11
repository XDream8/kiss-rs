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
use std::io::{self, BufReader, Read, Result};
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
pub fn prompt(log_message: Option<String>) {
    if let Some(log_message) = log_message {
        log!(log_message);
    }

    // Ask for confirmation if extra packages need to be built.
    log!("Continue?:", "Press Enter to continue or Ctrl+C to abort");

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
    let mut f: File = File::open(path)?;
    let mut s: String = String::new();
    match f.read_to_string(&mut s) {
        Ok(_) => Ok(s),
        Err(e) => Err(e),
    }
}

#[inline]
pub fn read_a_files_lines(
    file_path: impl AsRef<Path> + AsRef<std::ffi::OsStr>,
) -> Result<Vec<String>> {
    if Path::new(&file_path).exists() {
        let f: File = File::open(file_path)?;
        let buf: BufReader<File> = BufReader::new(f);
        let lines: Vec<String> = buf.lines().map_while(Result::ok).collect();

        return Ok(lines);
    }

    Ok(vec![])
}

#[inline]
pub fn mkcd(folder_name: impl AsRef<Path> + AsRef<std::ffi::OsStr> + AsRef<str>) {
    fs::create_dir_all(&folder_name).expect("Failed to create folder");
    env::set_current_dir(&folder_name).expect("Failed to change directory");
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
    match env::var(env) {
        Ok(v) => v,
        _ => default_value,
    }
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

                Some(path.clone())
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
            let mut source = source.clone();
            let mut dest = String::new();

            // consider user-given folder name
            if source.contains(' ') {
                let source_parts: Vec<String> = source.split(' ').map(|l| l.to_owned()).collect();
                source = source_parts.first().unwrap().to_owned();
                dest = source_parts
                    .last()
                    .unwrap()
                    .to_owned()
                    .trim_end_matches('/')
                    .to_owned();
            } else if !source.contains("git+") && !source.contains("http") {
                dest = source.clone();
            }

            (source, dest)
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

// run action as root
// pub fn run_action_as_root(action: Vec<&str>, force: bool) {
//     let command: String = if let Some(exe_path) = env::current_exe().ok() {
//         exe_path.to_string_lossy().to_string()
//     } else {
//         "Failed to get the full path of the running executable.".to_string()
//     };

//     // Execute the command with elevated privileges using `as_user`.
//     if let Err(_) = as_user(KISS_SU.to_string(), "root", command.as_str(), action, force) {
//         // Handle exit signal
//         process::exit(0);
//     } else {
//         process::exit(0);
//     }

// }

pub fn am_owner(file_or_dir: &str) -> Result<bool> {
    let metadata = fs::metadata(file_or_dir)?;
    let current_uid = unsafe { getuid() };
    Ok(metadata.uid() == current_uid)
}

// fn as_user(cmd_su: String, user: &str, command: &str,args: Vec<&str>, force: bool) -> Result<()> {
//     println!("Using '{}' (to become {})", cmd_su, user);

//     // create args vector but don’t initialize
//     let cmd_args: Vec<&str> = if cmd_su == "/usr/bin/su" {
//         vec![user, "-c"]
//     } else {
//         vec!["-u", user, "--"]
//     };

//     let mut child_cmd = Command::new(cmd_su.clone());

//     // Set the necessary environment variables for the child process.
//     let env_vars = [
//         "LOGNAME",
//         "HOME",
//         "XDG_CACHE_HOME",
//         "KISS_CHOICE",
//         "KISS_COMPRESS",
//         "KISS_FORCE",
//         "KISS_HOOK",
//         "KISS_TMPDIR",
//         "_KISS_LVL",
//     ];
//     for var in env_vars {
//         if let Ok(val) = env::var(var) {
//             child_cmd.env(var, val);
//         }
//     }

//     if force {
//         // first convert bool to u8. then convert it to string
//         let force_string: String = (force as u8).to_string();
//         child_cmd.env("KISS_FORCE", force_string);
//     } else {
//         child_cmd.env("KISS_FORCE", &*KISS_FORCE);
//     }
//     child_cmd.env("KISS_ROOT, &*KISS_ROOT);
//     child_cmd.env("KISS_PATH", &*KISS_PATH.join(":"));
//     child_cmd.env("KISS_PID", &*KISS_PID.to_string());

//     let mut child: Child = child_cmd
//         .args(cmd_args)
//         .arg(command)
//         .args(args)
//         .stdin(Stdio::inherit())
//         .stdout(Stdio::inherit())
//         .stderr(Stdio::inherit())
//         .spawn()
//         .expect("failed to execute child");

//     let ecode: ExitStatus = child.wait().expect("failed to wait on child");

//     if ecode.success() {
//         Ok(())
//     } else {
//         Err(io::Error::new(
//             io::ErrorKind::Other,
//             format!("Failed to execute '{}'", cmd_su),
//         ))
//     }
// }
