use shared_lib::get_directory_name;
use shared_lib::globals::Config;
use shared_lib::signal::pkg_clean;
// logging
use shared_lib::{die, log};

use checksum_lib::pkg_checksum_gen;
use search_lib::pkg_find_path;
use source_lib::pkg_source;

use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;

pub fn pkg_checksum(config: &Config, package: &str) {
    pkg_source(config, package, true, false);

    let repo_dir: String = pkg_find_path(config, package, None)
        .unwrap_or_else(|| die!(package.to_owned() + ":", "Failed to get version"))
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
