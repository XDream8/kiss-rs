// file libs
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use git2::{AutotagOption, FetchOptions, RemoteCallbacks, Repository};

// logging functions
use crate::shared_lib::signal::pkg_clean;
use crate::{die, log};

// thread
use crate::iter;
#[cfg(feature = "threading")]
use rayon::iter::ParallelIterator;

use crate::search_lib::{pkg_find_path, pkg_find_version};

use crate::shared_lib::globals::Config;
use crate::shared_lib::{is_symlink, mkcd, read_sources, remove_chars_after_last, tmp_file};

// tar
use std::fs;
use std::io::Read;
use tar::{Archive, Builder, Header};

#[cfg(feature = "bzip2")]
use bzip2::{read::BzDecoder, write::BzEncoder};
#[cfg(feature = "gzip")]
use flate2::{read::GzDecoder, write::GzEncoder};
#[cfg(feature = "lz4")]
use lzzzz::lz4f::{ReadDecompressor, WriteCompressor};
#[cfg(feature = "xz2")]
use xz2::{read::XzDecoder, write::XzEncoder};
#[cfg(feature = "zstd")]
use zstd::{stream::read::Decoder, stream::write::Encoder};

// for http client
use once_cell::sync::Lazy;
use std::time::Duration;
use ureq::{Agent, AgentBuilder, Response};

// reusable lazy initialized HTTP CLIENT
pub static HTTP_CLIENT: Lazy<Agent> = Lazy::new(|| {
    AgentBuilder::new()
        .timeout_read(Duration::from_secs(10))
        .timeout_write(Duration::from_secs(10))
        .build()
});

#[derive(PartialEq)]
pub enum SourceType {
    Git {
        source: String,
        destination: PathBuf,
    },
    Http {
        source: String,
        destination: PathBuf,
    },
    Cached(String),
    Unknown,
}

// get root directories of repositories and return them as a vector
pub fn get_repositories(repo_path: &[String]) -> Vec<String> {
    let mut repositories: Vec<String> = Vec::new();

    for repository in repo_path {
        let path: &Path = Path::new(&repository);
        if Repository::open(path).is_ok() {
            let path_str: String = path.to_string_lossy().to_string();
            if path.join(".git").exists() && !repositories.contains(&path_str) {
                repositories.push(path_str)
            }
        } else {
            let parent: &Path = path.parent().unwrap();
            let parent_str: String = parent.to_string_lossy().to_string();
            if parent.join(".git").exists() && !repositories.contains(&parent_str) {
                repositories.push(parent_str)
            }
        }
    }

    repositories
}

pub fn pkg_update_repo(repo_path: &str) -> Result<(), git2::Error> {
    let repository: Repository = Repository::open(repo_path)?;

    let cb = setup_callbacks();
    let mut remote = repository
        .find_remote("origin")
        .or_else(|_| repository.remote_anonymous("origin"))?;

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(cb);
    remote.fetch(&["origin"], Some(&mut fetch_options), None)?;

    {
        // If there are local objects (we got a thin pack), then tell the user
        // how many objects we saved from having to cross the network.
        let stats = remote.stats();
        if stats.local_objects() > 0 {
            println!(
                "\rReceived {}/{} objects in {} bytes (used {} local \
		 objects)",
                stats.indexed_objects(),
                stats.total_objects(),
                stats.received_bytes(),
                stats.local_objects()
            );
        } else {
            println!(
                "\rReceived {}/{} objects in {} bytes",
                stats.indexed_objects(),
                stats.total_objects(),
                stats.received_bytes()
            );
        }
    }

    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();

    // checkout_repo.checkout_tree(commit.as_object(), Some(&mut checkout_builder))?;
    repository.checkout_head(Some(&mut checkout_builder))?;

    Ok(())
}

fn setup_callbacks() -> RemoteCallbacks<'static> {
    let mut cb = RemoteCallbacks::new();
    // This callback gets called for each remote-tracking branch that gets
    // updated. The message we output depends on whether it's a new one or an
    // update.
    cb.update_tips(|refname, a, b| {
        if a.is_zero() {
            println!("[new]     {:20} {}", b, refname);
        } else {
            println!("[updated] {:10}..{:10} {}", a, b, refname);
        }
        true
    });

    // Here we show processed and total objects in the pack and the amount of
    // received data. Most frontends will probably want to show a percentage and
    // the download rate.
    cb.transfer_progress(|stats| {
        if stats.received_objects() == stats.total_objects() {
            print!(
                "Resolving deltas {}/{}\r",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            print!(
                "Received {}/{} objects ({}) in {} bytes\r",
                stats.received_objects(),
                stats.total_objects(),
                stats.indexed_objects(),
                stats.received_bytes()
            );
        }
        io::stdout().flush().unwrap();
        true
    });

    cb
}

// Given a line of input from the sources file, return an absolute
// path to the source if it already exists, error if not.
pub fn pkg_source_resolve(
    config: &Config,
    package_name: &str,
    repo_dir: &str,
    source: &String,
    dest: &String,
    print: bool,
) -> SourceType {
    let source_parts: Vec<String> = source.split('/').map(|e| e.to_owned()).collect();

    // get last element- repo name - for git
    let mut repo_name: String = source_parts.last().unwrap().to_owned();

    // both git and remote sources uses this dest
    let remote_dest: String = format!(
        "{}/{}/{}{}",
        config.sources_dir.to_string_lossy(),
        package_name,
        if !dest.is_empty() {
            format!("{}/", dest)
        } else {
            dest.to_string()
        },
        if &repo_name != dest && !repo_name.is_empty() {
            if let Some(index) = repo_name.find('#') {
                repo_name.truncate(index);
            }
            if let Some(index) = repo_name.find('@') {
                repo_name.truncate(index);
            }
            repo_name
        } else {
            "".to_owned()
        }
    );

    let source_type: SourceType = match source {
        // unwanted
        _ if source.starts_with('#') => SourceType::Unknown,
        // git source
        _ if source.starts_with("git+") => SourceType::Git {
            source: source.to_string(),
            destination: PathBuf::from(remote_dest),
        },
        // Remote source(cached)
        _ if source.contains("://") && Path::new(&remote_dest).exists() => {
            SourceType::Cached(remote_dest.to_string())
        }
        // remote source
        _ if source.contains("://") => SourceType::Http {
            source: source.to_string(),
            destination: PathBuf::from(remote_dest),
        },
        // Local relative dir
        _ if !repo_dir.is_empty()
            && Path::new(repo_dir).join(source.as_str()).join(".").exists() =>
        {
            let source: String = format!("{}/{}/.", repo_dir, source);
            SourceType::Cached(source)
        }
        // Local relative file
        _ if !repo_dir.is_empty() && Path::new(repo_dir).join(source.as_str()).exists() => {
            let source = format!("{}/{}", repo_dir, source);
            SourceType::Cached(source)
        }
        _ => SourceType::Unknown,
    };

    match &source_type {
        SourceType::Git {
            source: res,
            destination: _,
        }
        | SourceType::Http {
            source: res,
            destination: _,
        } => {
            if res.is_empty() {
                die!(package_name, "No local file:", source);
            }
        }
        SourceType::Cached(res) => {
            if print && (config.debug || config.verbose) {
                log!(package_name, "found", res);
            }
        }
        _ => {}
    }

    source_type
}

pub fn pkg_source(config: &Config, pkg: &str, skip_git: bool, print: bool) {
    let repo_dir: String = pkg_find_path(config, pkg, None)
        .unwrap_or_else(|| die!(pkg, "Failed to get package path"))
        .to_string_lossy()
        .to_string();

    let repo_name: String = pkg.to_string();

    let sources_file: PathBuf = Path::new(repo_dir.as_str()).join("sources");

    // Support packages without sources. Simply do nothing.
    if !sources_file.exists() {
        return;
    }

    if config.debug || config.verbose {
        log!(repo_name, "Reading sources");
    }

    let sources: Vec<(String, String)> = read_sources(sources_file.to_str().unwrap_or("sources"))
        .expect("Failed to read sources file");

    // Support packages with empty sources file. Simply do nothing

    iter!(sources).for_each(|(source, dest)| {
        let source_type = pkg_source_resolve(
            config,
            repo_name.as_str(),
            repo_dir.as_str(),
            source,
            dest,
            print,
        );

        match source_type {
            SourceType::Git {
                source,
                destination,
            } => {
                mkcd(remove_chars_after_last(&destination.to_string_lossy(), '/'));
                if !skip_git {
                    if let Err(err) =
                        pkg_source_git(&repo_name, &source, &destination.to_string_lossy(), true)
                    {
                        die!("Failed to fetch repository", err);
                    }
                }
            }
            SourceType::Http {
                source,
                destination,
            } => {
                mkcd(remove_chars_after_last(&destination.to_string_lossy(), '/'));
                if let Err(err) = pkg_source_url(config, &repo_name, &source, destination.as_path())
                {
                    die!("Failed to download file", err);
                }
            }

            _ => {}
        }
    });
}

// Experimental Function to clone git repos
// https://github.com/rust-lang/git2-rs/blob/master/examples/fetch.rs
pub fn pkg_source_git(
    package_name: &str,
    source: &String,
    des: &str,
    log: bool,
) -> Result<(), git2::Error> {
    let repo: Repository = match Repository::open(des) {
        Ok(repo) => repo,
        Err(_) => Repository::init(des)?,
    };
    let remote: &str = if !source.is_empty() {
        source.trim_start_matches("git+")
    } else {
        "origin"
    };

    // Figure out whether it's a named remote or a URL
    if log {
        log!(package_name, "Checking out:", remote);
    }
    let cb = setup_callbacks();
    let mut remote = repo
        .find_remote(remote)
        .or_else(|_| repo.remote_anonymous(remote))?;

    // Download the packfile and index it. This function updates the amount of
    // received data and the indexer stats which lets you inform the user about
    // progress.
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    fo.prune(git2::FetchPrune::On);
    fo.update_fetchhead(true);
    remote.download(&[] as &[&str], Some(&mut fo))?;

    {
        // If there are local objects (we got a thin pack), then tell the user
        // how many objects we saved from having to cross the network.
        let stats = remote.stats();
        if stats.local_objects() > 0 {
            println!(
                "\rReceived {}/{} objects in {} bytes (used {} local \
		 objects)",
                stats.indexed_objects(),
                stats.total_objects(),
                stats.received_bytes(),
                stats.local_objects()
            );
        } else {
            println!(
                "\rReceived {}/{} objects in {} bytes",
                stats.indexed_objects(),
                stats.total_objects(),
                stats.received_bytes()
            );
        }
    }

    // Disconnect the underlying connection to prevent from idling.
    remote.disconnect()?;

    // Update the references in the remote's namespace to point to the right
    // commits. This may be needed even if there was no packfile to download,
    // which can happen e.g. when the branches have been changed but all the
    // needed objects are available locally.
    remote.update_tips(None, true, AutotagOption::Unspecified, None)?;

    // checkout fetched content
    let reference = repo.find_reference("FETCH_HEAD")?;
    let commit = reference.peel_to_commit()?;
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();
    repo.checkout_tree(commit.as_object(), Some(&mut checkout_builder))?;

    Ok(())
}

// Function to download files
pub fn pkg_source_url(
    config: &Config,
    repo_name: &String,
    download_source: &String,
    download_dest: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    log!(repo_name, "Downloading:", download_source);

    let response: Response = HTTP_CLIENT.get(download_source).call()?;

    let total_size: u64 = response
        .header("Content-Length")
        .and_then(|length| length.parse::<u64>().ok())
        .unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut buffer: [u8; 8192] = [0; 8192];

    // get file_name from download_dest variable
    let file_name: String = download_dest
        .to_string_lossy()
        .split('/')
        .last()
        .unwrap()
        .to_owned();

    // tmp file
    let (mut tmp_file, tmp_file_path) = tmp_file(config, file_name.as_str(), "download")?;

    let mut response_reader = response.into_reader();

    while let Ok(bytes_read) = response_reader.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }

        downloaded += bytes_read as u64;

        print_progress(downloaded, total_size);

        tmp_file.write_all(&buffer[..bytes_read])?;
    }

    println!("\rDownloading {}: 100% (Completed)", download_source);

    // move tmp_file
    std::fs::rename(tmp_file_path, download_dest).expect("Failed to move tmp_file");

    Ok(())
}

pub fn print_progress(progress: u64, total_size: u64) {
    let formatted_progress: String = convert_bytes(progress);
    if total_size == 0 {
        print!("\rDownloading... ({}/Unknown)", formatted_progress);
    } else {
        let percent: f64 = (progress as f64 / total_size as f64) * 100.0;
        let formatted_total_size: String = convert_bytes(total_size);
        print!(
            "\rDownloading... {:.2}% ({}/{})",
            percent, formatted_progress, formatted_total_size
        );
    }
    io::stdout().flush().unwrap();
}

pub fn convert_bytes(bytes: u64) -> String {
    const UNIT: u64 = 1024;
    if bytes < UNIT {
        return format!("{} B", bytes);
    }
    let exp: u32 = (bytes as f64).log(UNIT as f64) as u32;
    let pre = "KMGTPE".chars().nth(exp as usize - 1).unwrap();
    let value: f64 = bytes as f64 / f64::powi(UNIT as f64, exp as i32);
    format!("{:.1} {}B", value, pre)
}

pub fn add_dirs_to_tar_recursive<W: Write>(
    builder: &mut Builder<W>,
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if the provided path is a directory
    if !dir.is_dir() {
        return Err("The provided path is not a directory.".into());
    }

    let entries: Vec<_> = fs::read_dir(dir)?.collect();

    for entry in entries {
        let entry = entry?;
        let entry_path: PathBuf = entry.path();
        let rel_file_path: &Path = entry_path.strip_prefix(dir)?;

        // file_type.is_symlink() follows symlink and gives wrong results
        if entry_path.is_symlink() || is_symlink(&entry_path) {
            // If it's a symlink, get the symlink target as a String
            let symlink_target: PathBuf = entry_path.read_link()?;
            let symlink_target_str: &str = symlink_target
                .to_str()
                .ok_or("Invalid UTF-8 in symlink target")?;

            // Create the symlink in the tar with the same target
            let mut header: Header = Header::new_ustar();
            header.set_path(rel_file_path)?;
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_link_name(symlink_target_str)?;
            builder.append(&header, &mut io::empty())?;
        } else if entry_path.is_dir() {
            builder.append_dir_all(rel_file_path, &entry_path)?;
        } else {
            let mut file = File::open(&entry_path)?;
            builder.append_file(rel_file_path, &mut file)?;
        }
    }

    Ok(())
}

pub fn create_tar_archive(
    file: &str,
    compress_path: &Path,
    compress_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // create tarball file
    let file: File = File::create(file)?;

    // encoder to use
    let encoder: Box<dyn Write> = match compress_type {
        #[cfg(feature = "gzip")]
        "gz" => Box::new(GzEncoder::new(file, flate2::Compression::default())),
        #[cfg(feature = "bzip2")]
        "bz2" => Box::new(BzEncoder::new(file, bzip2::Compression::default())),
        #[cfg(feature = "lz4")]
        "lz4" => Box::new(WriteCompressor::new(file, Default::default())?),
        #[cfg(feature = "xz2")]
        "xz" => Box::new(XzEncoder::new(file, 6)),
        // we use a BufWriter for zstd
        #[cfg(feature = "zstd")]
        "zst" => Box::new(Encoder::new(file, 0)?),
        _ => {
            die!("Unsupported compression type specified.");
        }
    };

    // create compressed tar archive
    let mut builder = Builder::new(encoder);
    add_dirs_to_tar_recursive(&mut builder, compress_path)?;

    builder.finish()?;

    Ok(())
}

// for creating tar archive
pub fn pkg_tar(config: &Config, pkg: &str) {
    log!(pkg, "Creating tarball");

    let pkg_ver: String =
        pkg_find_version(config, pkg, None).unwrap_or_else(|| die!(pkg, "Failed to get version"));
    let tar_file: String = format!(
        "{}/{}@{}.tar.{}",
        config.bin_dir.to_string_lossy(),
        pkg,
        pkg_ver,
        config.kiss_compress
    );
    let pkg_dir: PathBuf = config.pkg_dir.join(pkg);

    if let Err(err) = create_tar_archive(tar_file.as_str(), &pkg_dir, config.kiss_compress.as_str())
    {
        die!("Failed to create tarball:", err);
    } else {
        log!(pkg, "Successfully created tarball");
    }
}

// for extracting
pub fn pkg_source_tar(res: &String, extract_path: &Path, no_leading_dir: bool) {
    let file: File = File::open(res).expect("Failed to open tar file");
    let extension: Option<&str> = Path::new(res.as_str())
        .extension()
        .and_then(|ext| ext.to_str());
    let mut decoder: Box<dyn Read> = match extension {
        #[cfg(feature = "gzip")]
        Some("gz") => Box::new(GzDecoder::new(file)),
        #[cfg(feature = "bzip2")]
        Some("bz2") => Box::new(BzDecoder::new(file)),
        #[cfg(feature = "lz4")]
        Some("lz4") => {
            Box::new(ReadDecompressor::new(file).expect("Failed to decompress tar.lz4 archive"))
        }
        #[cfg(feature = "xz2")]
        Some("xz") => Box::new(XzDecoder::new(file)),
        #[cfg(feature = "zstd")]
        Some("zst") => Box::new(Decoder::new(file).expect("Failed to decompress tar.zst archive")),
        _ => return,
    };

    let mut archive: Archive<&mut Box<dyn std::io::Read>> = Archive::new(&mut decoder);

    // extract contents of tar directly to current dir
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap();

        // remove first level directory from dest
        let dest_path: PathBuf = if !no_leading_dir {
            extract_path.join(path)
        } else {
            extract_path.join(
                path.components()
                    .skip(1)
                    .collect::<std::path::PathBuf>()
                    .to_string_lossy()
                    .to_string(),
            )
        };

        if let Err(err) = entry.unpack(dest_path) {
            eprintln!("Failed to extract file: {}", err);
            continue;
        }
    }
}
