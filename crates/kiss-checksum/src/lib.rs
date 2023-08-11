use std::io::{Read, Result};
use std::path::{Path, PathBuf};
use shared_lib::read_a_files_lines;
use source_lib::pkg_source_resolve;
use shared_lib::read_sources;
// config
use shared_lib::globals::Config;
// logging
use shared_lib::signal::pkg_clean;
use shared_lib::{log, die};
// for b3sum hash generation
use blake3::Hasher;
use std::fs::File;
// threading
use shared_lib::iter;
#[cfg(feature = "threading")]
use rayon::iter::{ParallelIterator, IndexedParallelIterator};

pub fn pkg_checksum_gen(config: &Config, package_name: &str, repo_dir: &str) -> Vec<String> {
    let sources_path: PathBuf = Path::new(repo_dir).join("sources");
    let sources: Vec<(String, String)> = read_sources(sources_path.to_str().unwrap_or("sources"))
        .expect("Failed to read sources file");

    let hashes: Vec<_> = iter!(sources)
        .filter_map(|(source, dest)| {
            if !source.is_empty() && !source.starts_with("git+") {
                let (res, des) = pkg_source_resolve(config, package_name, repo_dir, source.clone(), dest.to_string(), false);

                // if it is a local source res equals to des
                if res == des {
                    Some(get_file_hash(&des))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .filter_map(|r| r.ok())
        .collect();

    hashes
}

pub fn get_file_hash(file_path: &str) -> Result<String> {
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let mut hash = Hasher::new().update(&buffer).finalize_xof();

    let mut hash_output = vec![0; 33];
    hash.fill(hash_output.as_mut_slice());

    Ok(faster_hex::hex_string(&hash_output))
}

pub fn pkg_verify(config: &Config, pkg: &str, repo_dir: String) {
    log!(pkg, "Verifying sources");

    let hashes: Vec<String> = pkg_checksum_gen(config, pkg, repo_dir.as_str());
    let checksums: Vec<String> =
        read_a_files_lines(format!("{}/checksums", repo_dir).as_str()).expect("No checksums file");

    iter!(hashes).zip(iter!(checksums)).for_each(|(element1, element2)| {
        println!("- {}\n+ {}", element2, element1);
        // checksum mismatch
        if element1 != element2 {
            die!(pkg, "Checksum mismatch");
        }
    });
}
