use shared_lib::globals::{get_repo_dir, get_repo_name};

use shared_lib::globals::Config;
// logging
use shared_lib::log;

use checksum_lib::pkg_checksum_gen;
use source_lib::pkg_source;

use std::io::{BufWriter, Write};
use std::path::Path;
use std::fs::OpenOptions;

pub fn pkg_checksum(config: &Config, package: &str) {
    pkg_source(config, package, true, false);

    let repo_dir = get_repo_dir();

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

        log!(get_repo_name().as_str(), "Generated checksums");
    } else {
        log!(get_repo_name().as_str(), "No sources needing checksums");
    }
}
