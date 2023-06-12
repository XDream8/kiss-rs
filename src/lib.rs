pub mod list;
pub mod search;

use std::fs;
use std::fs::{File, DirEntry};
use std::path::Path;
use std::io;
use std::io::prelude::*;

pub const INSTALLED_DIR: &'static str = "/var/db/kiss/installed";

pub fn cat(path: &Path) -> io::Result<String> {
    let mut f = File::open(path)?;
    let mut s = String::new();
    match f.read_to_string(&mut s) {
	Ok(_) => Ok(s),
	Err(e) => Err(e),
    }
}

pub fn read_a_dir_and_sort(path: &str) -> Vec<DirEntry> {
    let mut entries: Vec<_> = fs::read_dir(path).unwrap()
        .map(|p| p.unwrap())
        .collect();

    entries.sort_by_key(|dir| dir.path());

    return entries
}
