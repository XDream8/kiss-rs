pub mod checksum;
pub mod list;
pub mod search;

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

// Experimental
// pub static HTTP_CLIENT: Lazy<reqwest::Client> =
//     Lazy::new(|| reqwest::Client::builder().gzip(true).build().unwrap());

pub const SYS_DB: &'static str = "/var/db/kiss/installed";

pub static KISS_PATH: Lazy<Vec<String>> =Lazy::new(|| {
    let env_var: String = get_env_variable("KISS_PATH", SYS_DB);

    let mut path: Vec<String> = Vec::new();

    for repo in env_var.split(":").into_iter() {
	path.push(repo.to_owned());
    }

    // add installed packages directory
    path.push(SYS_DB.to_owned());

    // remove duplicates from paths
    let mut set = HashSet::new();
    path.retain(|x| set.insert(x.clone()));

    path
}
);

pub static KISS_TMP: Lazy<String> =Lazy::new(|| {
    let env_var: String = get_env_variable("KISS_TMP", format!("{}/kiss", get_env_variable("XDG_CACHE_HOME", "~/.cache")).as_str());

    env_var
}
);



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

pub fn read_a_file_lines(file_path: impl AsRef<Path>, error_msg: &str) -> Vec<String> {
    let f = File::open(file_path).expect(error_msg);
    let buf = BufReader::new(f);
    buf.lines()
        .map(|l| l.expect("Couldn't parse line"))
        .collect()
}

pub fn get_current_working_dir() -> Result<PathBuf> {
    env::current_dir()
}

pub fn get_env_variable(env: &str, default: &str) -> String {
    // get output of environment variable
    match env::var(env) {
	Ok(v) => v,
	_ => default.to_owned()
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

// Experimental Function to download files
// pub async fn fetch(uri: &str, body: &mut String) -> Result<(), reqwest::Error> {
//     let resp = HTTP_CLIENT.get(uri).send().await?;
//     if resp.status() != 200 {
//         eprintln!("{} ({}) {}: {}",
// 		  "fetching".red().bold(),
// 		  uri.yellow(),
// 		  "failed".red().bold(),
// 		  format!("{}", resp.status()).red().bold(),
//         );
//     }
//     else {
//         *body = resp.text().await?;
//     }
//     Ok(())
// }
