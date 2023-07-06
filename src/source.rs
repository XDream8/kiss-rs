// cli
use super::get_args;
use seahorse::Context;

// file libs
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use ureq::Response;
use git2::{AutotagOption, FetchOptions, RemoteCallbacks, Repository};

// logging functions
use super::die;
use super::log;

use super::read_sources;
use super::search::pkg_find_version;

// global variables
use super::HTTP_CLIENT;
use super::SRC_DIR;

use super::{get_repo_dir, get_repo_name};

use super::mkcd;
use super::remove_chars_after_last;
use super::tmp_file;

// decompress
use std::io::Read;
use tar::Archive;
use xz2::read::XzDecoder;
use flate2::read::GzDecoder;
use bzip2::read::BzDecoder;
use zstd::stream::read::Decoder;

// Given a line of input from the sources file, return an absolute
// path to the source if it already exists, error if not.
pub fn pkg_source_resolve(source: String, dest: String, print: bool) -> (String, String) {
    let repo_dir: String = get_repo_dir();

    let source_parts: Vec<String> = source.split("/").map(|e| e.to_owned()).collect();

    let package_name: String = get_repo_name();

    // get last element- repo name - for git
    let mut repo_name: String = source_parts.clone().last().unwrap().to_owned();

    // both git and remote sources uses this dest
    let _dest = format!(
	"{}/{}/{}{}",
	*SRC_DIR,
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
            && Path::new(format!("{}/{}/.", repo_dir, source).as_str()).exists() =>
        {
            let source = format!("{}/{}/.", repo_dir, source);
            (source.clone(), source)
        }
        // Local absolute dir
        _ if Path::new(format!("/{}", source.trim_end_matches("/")).as_str()).exists() => {
            let source = format!("/{}/.", source.trim_end_matches("/"));
            (source.clone(), source)
        }
        // Local relative file
        _ if !repo_dir.is_empty()
            && Path::new(format!("{}/{}", repo_dir, source).as_str()).exists() =>
        {
            let source = format!("{}/{}", repo_dir, source);
            (source.clone(), source)
        }
        // Local absolute file
        _ if Path::new(format!("/{}", source.trim_end_matches("/")).as_str()).exists() => {
            let source = format!("/{}", source.trim_end_matches("/"));
            (source.clone(), source)
        }
        _ => (String::new(), String::new()),
    };

    if _res.is_empty() || _des.is_empty() {
        die!(
            &package_name,
            format!("No local file '{}'", source).as_str()
        );
	// local
    } else if print && _res == _des {
        println!("found {}", _res);
    }
    (_res, _des)
}

pub fn pkg_source(pkg: &str, skip_git: bool, print: bool) {
    if !pkg.is_empty() {
	pkg_find_version(pkg, false);
    }

    let repo_name: String = get_repo_name();
    let repo_dir: String = get_repo_dir();

    let sources_file = format!("{}/sources", repo_dir);

    // Support packages without sources. Simply do nothing.
    if !Path::new(&sources_file).exists() {
        return;
    }

    log!(&repo_name, "Reading sources");

    let (sources, dests) = read_sources(sources_file.as_str()).expect("Failed to read sources file");

    for (source, dest) in sources.iter().zip(dests.unwrap().iter()) {
	let (res, des) = pkg_source_resolve(source.clone(), dest.clone(), print);

	mkcd(remove_chars_after_last(&des, '/'));

	// if it is a local source both res and des are set to the same value
	if res != des {
	    if !skip_git && res.starts_with("git+") {
		// place holder
		pkg_source_git(&repo_name, res.as_str(), des.as_str()).expect("Failed to fetch contents of repository");
	    } else if !res.starts_with("git+") {
		pkg_source_url(&res, Path::new(&des)).unwrap_or_else(|err| die!("Failed to download file: ", format!("{err}").as_str()));
	    }
	}
    }
}

// Experimental Function to clone git repos
// https://github.com/rust-lang/git2-rs/blob/master/examples/fetch.rs
pub fn pkg_source_git(package_name: &str, source: &str, des: &str) -> Result<(), git2::Error> {
    let repo = Repository::init(des)?;
    let remote: &str = match source {
	_ if !source.is_empty() => source.split("git+").last().unwrap(),
	_ => "origin",
    };

    // Figure out whether it's a named remote or a URL
    log!(package_name, format!("Checking out {}", remote));
    let mut cb = RemoteCallbacks::new();
    let mut remote = repo
        .find_remote(remote)
        .or_else(|_| repo.remote_anonymous(remote))?;
    cb.sideband_progress(|data| {
        print!("remote: {}", std::str::from_utf8(data).unwrap());
        io::stdout().flush().unwrap();
        true
    });

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
    checkout_repo.checkout_tree(commit.as_object(), None)?;

    Ok(())
}

    pub fn pkg_source_tar(res: String, no_leading_dir: bool) {
	let file: File = File::open(res.clone()).expect("Failed to open tar file");
	let extension: Option<&str> = Path::new(res.as_str()).extension().and_then(|ext| ext.to_str());
	let mut decoder: Box<dyn Read> = match extension {
	    Some("gz") => Box::new(GzDecoder::new(file)),
	    Some("xz") => Box::new(XzDecoder::new(file)),
	    Some("bz2") => Box::new(BzDecoder::new(file)),
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
	if no_leading_dir == false {
	    dest_path = dest_path.join(path);
	} else {
	    dest_path = dest_path.join(path.components().skip(1).collect::<std::path::PathBuf>().display().to_string());
	}

	if let Err(err) = entry.unpack(dest_path) {
	    eprintln!("Failed to extract file: {}", err);
	    continue
	}
    }
}

// Function to download files
pub fn pkg_source_url(
    download_source: &str,
    download_dest: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_name: String = get_repo_name();

    log!(
        &repo_name,
        format!("Downloading {}", download_source).as_str()
    );

    let response: Response = HTTP_CLIENT.get(download_source).call()?;

    let total_size: u64 = response
        .header("Content-Length")
        .and_then(|length| length.parse::<u64>().ok())
	.unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut buffer: [u8; 8192] = [0; 8192];

    // get file_name from download_dest variable
    let file_name: String = format!("{}", download_dest.display())
	.split("/")
	.last()
	.unwrap()
	.to_owned();

    // tmp file
    let (mut tmp_file, tmp_file_path) = tmp_file(file_name.as_str(), "download")?;

    let mut response_reader = response.into_reader();

    while let Ok(bytes_read) = response_reader.read(&mut buffer) {
	if bytes_read == 0 {
	    break
	}

	downloaded += bytes_read as u64;

	print_progress(downloaded, total_size);

	tmp_file.write_all(&buffer[..bytes_read])?;
    }

    println!("\rDownloading {}: 100% (Completed)", download_source);

    // move tmp_file
    std::fs::rename(tmp_file_path, download_dest)
        .expect("Failed to move tmp_file");

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

pub fn download_action(c: &Context) {
    let packages: Vec<&str> = get_args(&c);

    if !packages.is_empty() {
        for package in packages {
	    pkg_source(package, false, true);
        }
    } else {
        pkg_source("", false, true);
    }
}
