// cli
use seahorse::Context;
use super::get_args;
use super::search::pkg_find;

// file libs
use std::path::Path;
use std::fs::File;
use std::io::{self, Write};

use reqwest::header::CONTENT_LENGTH;

// use std::process::Command;

// logging functions
use super::die;
use super::log;

use super::search::pkg_find_version;
use super::read_a_files_lines;

// global variables
use super::HTTP_CLIENT;
use super::SRC_DIR;
use super::TMP_DIR;

use super::get_repo_dir;
use super::get_repo_name;

use super::mkcd;
use super::remove_chars_after_last;

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
        die(
            &package_name,
            format!("No local file '{}'", source).as_str(),
        );
    // local
    } else if print && _res == _des {
        println!("found {}", _res);
    }
    (_res, _des)
}

pub fn pkg_source(skip_git: bool, print: bool) {
    let repo_name: String = get_repo_name();

    pkg_find_version(&repo_name, false);

    let repo_dir: String = get_repo_dir();

    let sources_file = format!("{}/sources", repo_dir);

    // Support packages without sources. Simply do nothing.
    if !Path::new(&sources_file).exists() {
        return;
    }

    log(&repo_name, "Reading sources");

    let sources: Vec<String> = read_a_files_lines(sources_file, "ERROR");

    for source in sources {
        let mut source_clone = source.clone();
        let mut dest = String::new();

        // consider user-given folder name
        if source_clone.contains(" ") {
            let source_parts: Vec<String> = source_clone.split(" ").map(|l| l.to_owned()).collect();
            source_clone = source_parts.first().unwrap().to_owned();
            dest = source_parts
                .last()
                .unwrap()
                .to_owned()
                .trim_end_matches('/')
                .to_owned();
        }

        let (res, des) = pkg_source_resolve(source_clone, dest, print);

        mkcd(remove_chars_after_last(&des, '/'));

        // if it is a local source both res and des are set to the same value
        if res != des {
            if !skip_git && res.contains("git+") {
                // place holder
                die("ERR", "");
            } else {
                pkg_source_url(&res, Path::new(&des));
            }
        }
    }
}

// Experimental Function to clone git repos
// TODO: finish this function
// pub fn pkg_source_git(package_name: String, source: String) {
//     let mut com = source.clone();
//     if let Some(index) = com.find('#') {
// 	com.truncate(index);
//     }
//     if let Some(index) = com.find('@') {
// 	com.truncate(index);
//     }

//     log(&package_name, format!("Checking out {}",
// 			       if !com.is_empty() {
// 				   &com
// 			       }
// 			       else {
// 				   "FETCH_HEAD"
// 			       }).as_str()
//     );

//     if !Path::new(".git").exists() {
// 	let output = Command::new("git")
// 	    .arg("init")
// 	    .output();
//     }

// com=${2##*[@#]}
// com=${com#"${2%[#@]*}"}

// git remote set-url origin "${2%[#@]*}" 2>/dev/null ||
//     git remote add origin "${2%[#@]*}"

// 	git fetch --depth=1 origin "$com"
// 	git reset --hard FETCH_HEAD
// }

// Experimental Function to download files
#[tokio::main]
pub async fn pkg_source_url(
    download_source: &str,
    download_dest: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_name: String = get_repo_name();

    log(
        &repo_name,
        format!("Downloading {}", download_source).as_str(),
    );

    let mut response = HTTP_CLIENT.get(download_source).send().await?;

    let total_size = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let mut downloaded = 0;

    // get file_name from download_dest variable
    let file_name = format!("{}", download_dest.display())
        .split("/")
        .last()
        .unwrap()
        .to_owned();

    // tmp file
    let tmp_file_name = format!("{}-download", file_name);
    let tmp_file_path = Path::new(&*TMP_DIR).join(tmp_file_name);
    let mut tmp_file = File::create(&tmp_file_path)?;

    while let Some(chunk) = response.chunk().await? {
        let chunk_size = chunk.len() as u64;

        downloaded += chunk_size;

        print_progress(downloaded, total_size);

        tmp_file.write_all(&chunk)?;
    }

    println!("\rDownloading {}: 100% (Completed)", download_source);

    // move tmp_file
    std::fs::rename(tmp_file_path.to_string_lossy().into_owned(), download_dest)
        .expect("Failed to move tmp_file");

    Ok(())
}

pub fn print_progress(progress: u64, total_size: u64) {
    let percent = (progress as f64 / total_size as f64) * 100.0;
    let formatted_progress = convert_bytes(progress);
    let formatted_total_size = convert_bytes(total_size);
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
    let exp = (bytes as f64).log(UNIT as f64) as u32;
    let pre = "KMGTPE".chars().nth(exp as usize - 1).unwrap();
    let value = bytes as f64 / f64::powi(UNIT as f64, exp as i32);
    format!("{:.1} {}B", value, pre)
}

pub fn download_action(c: &Context) {
    let packages: Vec<&str> = get_args(&c);

    if !packages.is_empty() {
	for package in packages {
	    let pac = pkg_find(package, false);
	    if !pac.is_empty() {
		pkg_source(false, true);
	    } else {
		log(get_repo_name().as_str(), "package not found");
	    }
	}
    } else {
	pkg_source(false, true);
    }
}
