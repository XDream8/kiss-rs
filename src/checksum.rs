use seahorse::Context;

use super::get_args;
use super::get_repo_dir;
use super::get_repo_name;

use super::read_a_files_lines;
use super::read_sources;

use super::{log, die};

use super::source::pkg_source;
use super::source::pkg_source_resolve;

// for b3sum hash generation
use blake3::Hasher;
use std::fs::{File, OpenOptions};

use std::io::{BufWriter, Read, Result, Write};
use std::path::Path;

pub fn pkg_checksum(package: &str) {
    pkg_source(package, true, false);

    let repo_dir = get_repo_dir();

    if !Path::new(repo_dir.as_str()).join("sources").exists() {
	return
    }

    let hashes: Vec<String> = pkg_checksum_gen(repo_dir.clone());

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

	log!(get_repo_name().as_str(), "Generated checksums");
    } else {
	log!(get_repo_name().as_str(), "No sources needing checksums");
    }
}

pub fn pkg_checksum_gen(repo_dir: String) -> Vec<String> {
    let sources_path = format!("{}/sources", repo_dir);
    let mut hashes: Vec<String> = Vec::new();
    let (sources, dests) = read_sources(sources_path.as_str()).expect("Failed to read sources file");

    for (source, dest) in sources.iter().zip(dests.unwrap().iter()) {
	if !source.is_empty() && !source.starts_with("git+") {
	    let (res, des) = pkg_source_resolve(source.clone(), dest.clone(), false);

	    // if it is a local source res equals to des
	    if res == des {
		hashes.push(get_file_hash(&des).expect("Failed to generate checksums"));
	    }
	}
    }

	hashes
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

pub fn pkg_verify(pkg: &str, repo_dir: String) {
    log!(pkg, "Verifying sources");

    let hashes: Vec<String> = pkg_checksum_gen(repo_dir.clone());
    let checksums: Vec<String> = read_a_files_lines(format!("{}/checksums", repo_dir).as_str()).expect("No checksums file");

    for (element1, element2) in hashes.iter().zip(checksums.iter()) {
	println!("- {}\n+ {}", element2, element1);
	// checksum mismatch
	if element1 != element2 {
	    die!(pkg, "Checksum mismatch");
	}
    }

}

pub fn checksum_action(c: &Context) {
    let packages: Vec<&str> = get_args(c);

    // search package
    if !packages.is_empty() {
        for package in packages {
            pkg_checksum(package);
        }
    } else {
        pkg_checksum("");
    }
}
