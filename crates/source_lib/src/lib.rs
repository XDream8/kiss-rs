// file libs
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use git2::{AutotagOption, FetchOptions, RemoteCallbacks, Repository};

// logging functions
use shared_lib::signal::pkg_clean;
use shared_lib::{die, log};

// thread
#[cfg(feature = "threading")]
use rayon::iter::ParallelIterator;
use shared_lib::iter;

use search_lib::{pkg_find, pkg_find_version};

use shared_lib::globals::{get_repo_dir, get_repo_name, Config};
use shared_lib::{is_symlink, mkcd, read_sources, remove_chars_after_last, tmp_file};

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

// Given a line of input from the sources file, return an absolute
// path to the source if it already exists, error if not.
pub fn pkg_source_resolve(
    config: &Config,
    package_name: &str,
    repo_dir: &str,
    source: String,
    dest: String,
    print: bool,
) -> (String, String) {
    let source_parts: Vec<String> = source.split('/').map(|e| e.to_owned()).collect();

    // get last element- repo name - for git
    let mut repo_name: String = source_parts.clone().last().unwrap().to_owned();

    // both git and remote sources uses this dest
    let _dest = format!(
        "{}/{}/{}{}",
        config.sources_dir.to_string_lossy(),
        package_name,
        if !dest.is_empty() {
            format!("{}/", dest)
        } else {
            dest
        },
        if !repo_name.is_empty() {
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

    // let remote_dest = format!("{}", *SRC_DIR, package_name, );

    let source_clone = source.clone();

    let (_res, _des) = match source {
        // unwanted
        _ if source.starts_with('#') => ("".to_owned(), "".to_owned()),
        // git source
        _ if source.starts_with("git+") => (source_clone, _dest),
        // Remote source(cached)
        _ if source.contains("://") && Path::new(&_dest).exists() => (_dest.clone(), _dest),
        // Remote source
        _ if source.contains("://") => (source_clone, _dest),
        // Local relative dir
        _ if !repo_dir.is_empty()
            && Path::new(repo_dir).join(source.as_str()).join(".").exists() =>
        {
            let source = format!("{}/{}/.", repo_dir, source);
            (source.clone(), source)
        }
        // Local absolute dir
        _ if Path::new("/").join(source.trim_end_matches('/')).exists() => {
            let source = format!("/{}/.", source.trim_end_matches('/'));
            (source.clone(), source)
        }
        // Local relative file
        _ if !repo_dir.is_empty() && Path::new(repo_dir).join(source.as_str()).exists() => {
            let source = format!("{}/{}", repo_dir, source);
            (source.clone(), source)
        }
        // Local absolute file
        _ if Path::new("/").join(source.trim_end_matches('/')).exists() => {
            let source = format!("/{}", source.trim_end_matches('/'));
            (source.clone(), source)
        }
        _ => (String::new(), String::new()),
    };

    if _res.is_empty() || _des.is_empty() {
        die!(format!("{}:", package_name), "No local file:", source);
        // local
    } else if print && _res == _des {
        log!(format!("{}:", package_name), "found", _res);
    }
    (_res, _des)
}

pub fn pkg_source(config: &Config, pkg: &str, skip_git: bool, print: bool) {
    let repo_name: String = if !pkg.is_empty() {
        pkg.to_string()
    } else {
        get_repo_name()
    };
    let repo_dir: String = if !pkg.is_empty() {
        pkg_find(config, pkg, false, false, false)
    } else {
        get_repo_dir()
    };

    let sources_file: PathBuf = Path::new(repo_dir.as_str()).join("sources");

    // Support packages without sources. Simply do nothing.
    if !sources_file.exists() {
        return;
    }

    if config.debug {
        log!(&repo_name, "Reading sources");
    }

    let sources: Vec<(String, String)> = read_sources(sources_file.to_str().unwrap_or("sources"))
        .expect("Failed to read sources file");

    iter!(sources).for_each(|(source, dest)| {
        let (res, des) = pkg_source_resolve(
            config,
            repo_name.as_str(),
            repo_dir.as_str(),
            source.clone(),
            dest.clone(),
            print,
        );

        mkcd(remove_chars_after_last(&des, '/'));

        // if it is a local source both res and des are set to the same value
        if res != des {
            if !skip_git && res.starts_with("git+") {
                if let Err(err) = pkg_source_git(&repo_name, res.as_str(), des.as_str()) {
                    die!("Failed to fetch repository:", err);
                }
            } else if res.starts_with("https://") || res.starts_with("http://") {
                if let Err(err) = pkg_source_url(config, &res, Path::new(&des)) {
                    die!("Failed to download file:", err);
                }
            }
        }
    });
}

// Experimental Function to clone git repos
// https://github.com/rust-lang/git2-rs/blob/master/examples/fetch.rs
pub fn pkg_source_git(package_name: &str, source: &str, des: &str) -> Result<(), git2::Error> {
    let repo = match Repository::open(des) {
        Ok(repo) => repo,
        Err(_) => Repository::init(des)?,
    };
    let remote = if !source.is_empty() {
        source.trim_start_matches("git+")
    } else {
        "origin"
    };

    // Figure out whether it's a named remote or a URL
    log!(package_name, "Checking out:", remote);
    let mut cb = RemoteCallbacks::new();
    let mut remote = repo
        .find_remote(remote)
        .or_else(|_| repo.remote_anonymous(remote))?;

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
    let checkout_repo = Repository::open(des)?;
    let reference = checkout_repo.find_reference("FETCH_HEAD")?;
    let commit = reference.peel_to_commit()?;
    // force checkout
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();

    checkout_repo.checkout_tree(commit.as_object(), Some(&mut checkout_builder))?;

    Ok(())
}

// Function to download files
pub fn pkg_source_url(
    config: &Config,
    download_source: &str,
    download_dest: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_name: String = get_repo_name();

    log!(&repo_name, "Downloading:", download_source);

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
    let percent: f64 = (progress as f64 / total_size as f64) * 100.0;
    let formatted_progress: String = convert_bytes(progress);
    let formatted_total_size: String = convert_bytes(total_size);
    print!(
        "\rDownloading... {:.2}% ({}/{})",
        percent, formatted_progress, formatted_total_size
    );
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
        if is_symlink(&entry_path) {
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

    let pkg_ver: String = pkg_find_version(config, pkg, false);
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
pub fn pkg_source_tar(res: String, no_leading_dir: bool) {
    let file: File = File::open(&res).expect("Failed to open tar file");
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

        let mut dest_path: PathBuf = Path::new(".").to_path_buf();
        // remove first level directory from dest
        if !no_leading_dir {
            dest_path = dest_path.join(path);
        } else {
            dest_path = dest_path.join(
                path.components()
                    .skip(1)
                    .collect::<std::path::PathBuf>()
                    .to_string_lossy()
                    .to_string(),
            );
        }

        if let Err(err) = entry.unpack(dest_path) {
            eprintln!("Failed to extract file: {}", err);
            continue;
        }
    }
}
