use crate::search_lib::pkg_find_path;
use crate::shared_lib::{get_directory_name, globals::Config, read_a_files_lines, read_sources};
use crate::source_lib::{pkg_source, pkg_source_resolve, SourceType};
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Read, Result, Write},
    path::{Path, PathBuf},
};
// logging
use crate::shared_lib::signal::pkg_clean;
use crate::{die, log};
// for b3sum hash generation
use blake3::Hasher;

// threading
use crate::iter;
#[cfg(feature = "threading")]
use rayon::iter::{IndexedParallelIterator, ParallelIterator};

pub fn pkg_checksum_gen(config: &Config, package_name: &str, repo_dir: &str) -> Vec<String> {
    let sources_path: PathBuf = Path::new(repo_dir).join("sources");
    let sources: Vec<(String, String)> = read_sources(sources_path.to_str().unwrap_or("sources"))
        .expect("Failed to read sources file");

    let hashes: Vec<_> = iter!(sources)
        .filter_map(|(source, dest)| {
            if !source.is_empty() && !source.starts_with("git+") {
                let (source_type, _, des) =
                    pkg_source_resolve(config, package_name, repo_dir, source, dest, false);

                // if it is a local source
                if source_type == SourceType::Cached {
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

pub fn pkg_verify(config: &Config, pkg: &str, repo_dir: &String) {
    if config.debug || config.verbose {
        log!(pkg, "Verifying sources");
    }

    let hashes: Vec<String> = pkg_checksum_gen(config, pkg, repo_dir.as_str());
    let checksums: Vec<String> =
        read_a_files_lines(format!("{}/checksums", repo_dir).as_str()).expect("No checksums file");

    iter!(hashes)
        .zip(iter!(checksums))
        .for_each(|(element1, element2)| {
            if config.debug || config.verbose {
                println!("- {}\n+ {}", element2, element1);
            }
            // checksum mismatch
            if element1 != element2 {
                die!(pkg, "Checksum mismatch");
            }
        });
}

pub fn pkg_checksum(config: &Config, package: &str) {
    pkg_source(config, package, true, false);

    let repo_dir: String = pkg_find_path(config, package, None)
        .unwrap_or_else(|| die!(package, "Failed to get version"))
        .to_string_lossy()
        .to_string();
    let repo_name: &str = get_directory_name(&repo_dir);

    if !Path::new(repo_dir.as_str()).join("sources").exists() {
        return;
    }

    let hashes: Vec<String> = pkg_checksum_gen(config, package, repo_dir.as_str());

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

        log!(repo_name, "Generated checksums");
    } else {
        log!(repo_name, "No sources needing checksums");
    }
}
