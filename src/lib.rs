pub mod checksum;
pub mod list;
pub mod search;

//pub mod source;

use std::fs;
use std::fs::{File, DirEntry};
use std::path::{Path, PathBuf};
use std::io::prelude::*;
use std::io::{BufReader, Read, Result};
use blake3::Hasher;

// using this to remove duplicate path entries
use std::collections::HashSet;

use seahorse::Context;
use std::env;

// colored output
use colored::*;

use once_cell::sync::Lazy;

// Variables
// almost all global variables should be lazy


// Experimental
// pub static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| reqwest::Client::builder().gzip(true).build().unwrap());

pub static SYS_DB: &'static str = "/var/db/kiss/installed";

pub static HOME: Lazy<String> = Lazy::new(|| get_env_variable("HOME", String::new()));
pub static CACHE: Lazy<String> = Lazy::new(|| get_env_variable("XDG_CACHE_HOME", format!("{}/.cache", *HOME)));

// Cache
pub static CAC_DIR: Lazy<String> = Lazy::new(|| format!("{}/kiss", *CACHE));
pub static SRC_DIR: Lazy<String> = Lazy::new(|| format!("{}/sources", *CAC_DIR));
pub static LOG_DIR: Lazy<String> = Lazy::new(|| format!("{}/logs", *CAC_DIR));
pub static BIN_DIR: Lazy<String> = Lazy::new(|| format!("{}/bin", *CAC_DIR));

pub static PROC: Lazy<String> = Lazy::new(|| format!("{}/proc/{}", *CAC_DIR, *KISS_PID));
pub static MAK_DIR: Lazy<String> = Lazy::new(|| format!("{}/build", *PROC));
pub static PKG_DIR: Lazy<String> = Lazy::new(|| format!("{}/pkg", *PROC));
pub static TAR_DIR: Lazy<String> = Lazy::new(|| format!("{}/extract", *PROC));
pub static TMP_DIR: Lazy<String> = Lazy::new(|| format!("{}/tmp", *PROC));

pub static KISS_PID: Lazy<u32> = Lazy::new(|| std::process::id());
pub static KISS_TMP: Lazy<String> = Lazy::new(|| get_env_variable("KISS_TMP", format!("{}/kiss", *CACHE)));
pub static KISS_DEBUG: Lazy<String> = Lazy::new(|| get_env_variable("KISS_DEBUG", "0".to_owned()));
pub static KISS_LVL: Lazy<String> = Lazy::new(|| get_env_variable("KISS_LVL", "1".to_owned()));

pub static KISS_PATH: Lazy<Vec<String>> = Lazy::new(|| {
    let env_var: String = get_env_variable("KISS_PATH", SYS_DB.to_owned());

    let mut path: Vec<String> = Vec::new();

    for repo in env_var.split(":").into_iter() {
	path.push(repo.to_owned());
    }

    // add installed packages directory
    path.push(SYS_DB.to_owned());

    // remove duplicates and empty entries from paths
    let mut set = HashSet::new();
    path.retain(|x| !x.is_empty() && set.insert(x.clone()));

    path

}
);

// Functions
pub fn create_tmp_dirs() -> i32 {
    let dirs = vec!(&*SRC_DIR, &*LOG_DIR, &*BIN_DIR, &*MAK_DIR, &*PKG_DIR, &*TAR_DIR, &*TMP_DIR);
    for dir in dirs {
	fs::create_dir_all(dir).expect("Failed to create directory");
    }

    0
}

pub fn pkg_clean() -> i32 {
    if *KISS_DEBUG == "0" {
	if *KISS_LVL == "1" {
	    fs::remove_dir_all(&*PROC).expect("Failed to remove directory");
	}
	else {
	    fs::remove_dir_all(&*TAR_DIR).expect("Failed to remove directory");
	}
    }

    0
}

pub fn get_args(c: &Context) -> Vec<&str> {
    let mut args: Vec<&str> = vec![];

    for arg in &c.args {
	args.push(arg.as_str())
    }

    args
}

pub fn cat(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut s = String::new();
    match f.read_to_string(&mut s) {
	Ok(_) => Ok(s),
	Err(e) => Err(e),
    }
}

pub fn read_a_files_lines(file_path: impl AsRef<Path>, error_msg: &str) -> Vec<String> {
    let f = File::open(file_path).expect(error_msg);
    let buf = BufReader::new(f);
    buf.lines()
	.map(|l| l.expect("Couldn't parse line"))
	.collect()
}

pub fn get_current_working_dir() -> Result<PathBuf> {
    env::current_dir()
}

pub fn get_env_variable(env: &str, default_value: String) -> String {
    // get output of environment variable
    match env::var(env) {
	Ok(v) => v,
	_ => default_value
    }
}

					    pub fn file_exists_in_current_dir(filename: &str) -> bool {
    get_current_working_dir().expect(format!("{}: Failed to get working dir", "ERROR".yellow()).as_str()).join(filename).exists()
}

pub fn read_a_dir_and_sort(path: &str) -> Vec<DirEntry> {
    let mut entries: Vec<_> = fs::read_dir(path).unwrap()
        .map(|p| p.unwrap())
        .collect();

    entries.sort_by_key(|dir| dir.path());

    return entries
}

pub fn get_file_hash(file_path: &str) -> Result<String> {
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let mut hash = Hasher::new()
        .update(&buffer)
        .finalize_xof();

    let mut hash_output = vec![0; 32];
    hash.fill(hash_output.as_mut_slice());

    Ok(hex::encode(hash_output))
}
