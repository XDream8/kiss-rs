use crate::{
    error::Error,
    package_info::Package,
    source::{pkg_download_source, Source, SourceType},
};
use blake3::Hasher;

use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

pub fn pkg_checksum_gen(sources: &Vec<Source>) -> Result<Vec<String>, Error> {
    sources
        .iter()
        .filter_map(|source| {
            // use filter_map
            if source.source_type == SourceType::Local || source.source_type == SourceType::Http {
                // if it is a local source
                source
                    .source_file_path
                    .iter() // use into_iter
                    .map(|source_path| get_file_hash(source_path))
                    .next()
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

pub fn get_file_hash(file_path: &Path) -> Result<String, Error> {
    let file = File::open(file_path)?;
    // wrap the file in a buffered reader
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    // read the file contents into the buffer
    reader.read_to_end(&mut buffer)?;

    let mut hash = Hasher::new().update(&buffer).finalize_xof();

    let mut hash_output = vec![0; 33];
    hash.fill(hash_output.as_mut_slice());

    Ok(faster_hex::hex_string(&hash_output))
}

pub fn pkg_checksum(package_info: &Package, tmp_dir: &Path) -> Result<(), Error> {
    // fetch sources
    pkg_download_source(&package_info.name, &package_info.sources, tmp_dir)?;

    let hashes: Vec<String> = pkg_checksum_gen(&package_info.sources)?;

    if !hashes.is_empty() {
        let checksums_file_in_repo_dir = package_info.package_repo_path.join("checksums");
        // create or recreate checksums file
        let checksums_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(checksums_file_in_repo_dir)?;

        // use a buffered writer for performance
        let mut writer = BufWriter::new(checksums_file);

        for hash in hashes {
            println!("{}", hash);
            writer.write_all(hash.as_bytes())?;
            writer.write_all(b"\n")?;
        }

        // ensure all data is written to the file
        writer.flush().expect("Failed to write to checksums file");

        println!("{}: Generated checksums", package_info.name);
    } else {
        println!("{}: No sources needing checksums", package_info.name);
    }

    Ok(())
}
