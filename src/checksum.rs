use seahorse::Context;

use super::get_args;
use super::get_repo_dir;
use super::get_repo_name;

use super::read_a_files_lines;

use super::log;

use super::search::pkg_find;

use super::source::pkg_source;
use super::source::pkg_source_resolve;

// for b3sum hash generation
use blake3::Hasher;
use std::fs::{File, OpenOptions};

use std::io::{BufWriter, Read, Result, Write};

pub fn pkg_checksum() {
    let repo_dir = get_repo_dir();
    let sources_path = format!("{}/sources", repo_dir);
    let sources: Vec<String> = read_a_files_lines(sources_path.as_str(), "No sources file");

    pkg_source(true, false);

    let mut hashes: Vec<String> = Vec::new();

    for source in sources {
	let mut source = source.clone();
	let mut dest = String::new();

	// consider user-given folder name
	if source.contains(" ") {
	    let source_parts: Vec<String> = source.split(" ").map(|l| l.to_owned()).collect();
	    source = source_parts.first().unwrap().to_owned();
	    dest = source_parts
		.last()
		.unwrap()
		.to_owned()
		.trim_end_matches('/')
		.to_owned();
	}

	let (res, des) = pkg_source_resolve(source, dest, false);

	// if it is a local source res equals to des
	if res == des && !res.contains("git+") {
	    hashes.push(get_file_hash(&des).expect("Failed to generate checksums"));
	}
    }

    if !hashes.is_empty() {
	// create or recreate checksums file
	let checksums_file = OpenOptions::new()
	    .write(true)
	    .truncate(true)
	    .create(true)
	    .open(format!("{}/checksums", repo_dir))
	    .expect("Failed to create or recreate checksums file");

	// use a buffered writer for performance
	let mut writer = BufWriter::new(checksums_file);

	for hash in hashes {
	    println!("{}", hash);
	    writer
		.write_all(hash.as_bytes())
		.expect("Failed to write to checksums file");
	    writer
		.write_all(b"\n")
		.expect("Failed to write to checksums file");
	}

		    // ensure all data is written to the file
		    writer.flush().expect("Failed to write to checksums file");

		    log(get_repo_name().as_str(), "Generated checksums");
		} else {
		    log(get_repo_name().as_str(), "No sources needing checksums");
		}
	    }

	    pub fn get_file_hash(file_path: &str) -> Result<String> {
		let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let mut hash = Hasher::new().update(&buffer).finalize_xof();

    let mut hash_output = vec![0; 33];
    hash.fill(hash_output.as_mut_slice());

    Ok(hex::encode(hash_output))
}

pub fn checksum_action(c: &Context) {
    let packages: Vec<&str> = get_args(&c);

    // search package
    if !packages.is_empty() {
	for package in packages {
	    let pac = pkg_find(package, false);
	    if !pac.is_empty() {
		pkg_checksum();
	    } else {
		log(get_repo_name().as_str(), "package not found");
	    }
	}
    } else {
	pkg_checksum();
    }
}
