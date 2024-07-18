use nix::unistd::{getuid, Uid, User};
use std::fs::{File, Metadata};
use std::io::{BufRead, BufReader, Error, ErrorKind};
use std::os::unix::prelude::MetadataExt;
use std::path::{Path, PathBuf};
use std::{env, fs};

#[inline]
pub fn get_current_working_dir() -> Result<PathBuf, Error> {
    match env::current_dir() {
        Ok(current_dir) => Ok(current_dir),
        Err(err) => Err(Error::new(
            ErrorKind::Other,
            format!("Error getting current working directory: {}", err),
        )),
    }
}

/// optimized cat function
#[inline]
pub fn cat(path: &Path) -> Result<String, Error> {
    let file_bytes: Vec<u8> = fs::read(path)?;
    let buffer: String = String::from_utf8_lossy(&file_bytes).into_owned();
    Ok(buffer)
}

#[inline]
pub fn read_a_files_lines(
    file_path: impl AsRef<Path> + AsRef<std::ffi::OsStr>,
) -> Result<Vec<String>, Error> {
    if Path::new(&file_path).exists() {
        let f: File = File::open(file_path)?;
        let buf: BufReader<File> = BufReader::new(f);
        let lines: Vec<String> = buf.lines().collect::<Result<_, _>>()?;
        Ok(lines)
    } else {
        Ok(vec![])
    }
}

#[inline]
pub fn read_a_dir_and_sort(
    path: impl AsRef<Path>,
    recursive: bool,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let folder_path = path.as_ref();
    let mut filtered_entries: Vec<PathBuf> = Vec::new();

    if folder_path.is_dir() {
        let entries: Result<Vec<_>, _> = fs::read_dir(folder_path)?
            .map(|entry| entry.map(|e| e.path()))
            .collect();

        // Handle the Result and collect entries
        filtered_entries.extend(entries?);

        if recursive {
            // Create a separate vector for subfolder entries
            let mut subfolder_entries: Vec<PathBuf> = Vec::new();

            // Recursively read and sort subdirectories
            for subfolder_entry in &filtered_entries {
                if subfolder_entry.is_dir() {
                    let entries = read_a_dir_and_sort(subfolder_entry, true)?;
                    subfolder_entries.extend(entries);
                }
            }

            // Extend filtered_entries with subfolder_entries
            filtered_entries.extend(subfolder_entries);
        }
    }

    // Sort the entries
    filtered_entries.sort();

    Ok(filtered_entries)
}

#[inline]
pub fn check_dir_ownership(file_or_dir: &Path) -> Result<(bool, Option<String>), Error> {
    let metadata: Metadata = fs::metadata(file_or_dir)?;
    let current_uid: Uid = getuid();

    if metadata.uid() == current_uid.into() {
        Ok((true, None))
    } else {
        let user_name: String = if let Ok(Some(user)) = User::from_uid(metadata.uid().into()) {
            user.name
        } else {
            String::from("unknown")
        };
        Ok((false, Some(user_name)))
    }
}

#[inline]
pub fn tmp_file(tmp_dir: &Path, name: &str, suffix: &str) -> Result<(File, PathBuf), Error> {
    let tmp_file_name: String = format!("{}-{}", name, suffix);
    let tmp_file_path: PathBuf = tmp_dir.join(tmp_file_name);
    let tmp_file: File = File::create(&tmp_file_path)?;

    Ok((tmp_file, tmp_file_path))
}
