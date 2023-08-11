use seahorse::Context;

use crate::{get_current_working_dir, get_directory_name, get_env_variable};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

// signal handling
use crate::signal::handle_sigint;
use libc::{signal, SIGINT, SIGTERM};

// for http client
// use std::time::Duration;
// use ureq::{Agent, AgentBuilder};

// define a struct to hold shared variables
#[derive(Debug, Default, Clone)]
pub struct Config {
    pub choice: bool,
    pub debug: bool,
    pub force: bool,
    pub prompt: bool,
    pub strip: bool,
    pub lvl: u8,
    pub pid: u32,
    pub proc: PathBuf,
    // kiss_*
    pub kiss_cache_dir: PathBuf,
    pub kiss_compress: String,
    pub kiss_path: Vec<String>,
    pub kiss_root: PathBuf,
    pub kiss_tmp_dir: PathBuf,
    // temoporary directories
    pub mak_dir: PathBuf,
    pub pkg_dir: PathBuf,
    pub tar_dir: PathBuf,
    pub tmp_dir: PathBuf,
    // base dirs
    pub sources_dir: PathBuf,
    pub log_dir: PathBuf,
    pub bin_dir: PathBuf,
    // database
    pub cho_db: String,
    pub pkg_db: String,
    pub sys_db: PathBuf,
}

// implement a builder function
impl Config {
    pub fn new() -> Self {
        let home: String = get_env_variable("HOME", String::new());
        let cache: String = get_env_variable("XDG_CACHE_HOME", format!("{}/.cache", home));
        let pid: u32 = std::process::id();

        // get env variables - if they are not found default_value is used
        let kiss_cache_dir: PathBuf = {
            let env: String = get_env_variable("KISS_CACHE_DIR", format!("{}/kiss", cache));
            PathBuf::from(env)
        };
        let kiss_compress: String = get_env_variable("KISS_COMPRESS", "gz".to_owned());
        let kiss_root: PathBuf = {
            let env: String = get_env_variable("KISS_ROOT", "/".to_owned());
            PathBuf::from(env)
        };
        let kiss_tmp_dir: PathBuf = {
            let env: String = get_env_variable("KISS_TMPDIR", format!("{}/kiss", cache));
            PathBuf::from(env)
        };

        // Cache stuff
        let sources_dir: PathBuf = kiss_cache_dir.join("sources");
        let log_dir: PathBuf = kiss_cache_dir.join("logs");
        let bin_dir: PathBuf = kiss_cache_dir.join("bin");

        // tmpdir stuff
        let proc: PathBuf = kiss_tmp_dir.join("proc").join(pid.to_string().as_str());
        let mak_dir: PathBuf = proc.join("build");
        let pkg_dir: PathBuf = proc.join("pkg");
        let tar_dir: PathBuf = proc.join("extract");
        let tmp_dir: PathBuf = proc.join("tmp");

        // db stuff
        let cho_db: String = "var/db/kiss/choices".to_string();
        let pkg_db: String = "var/db/kiss/installed".to_string();
        let sys_db: PathBuf = kiss_root.join(&pkg_db);

        // and lastly kiss path
        let kiss_path: Vec<String> = {
            let env_var: String = get_env_variable("KISS_PATH", String::new());

            let mut path: Vec<String> = Vec::new();

            for repo in env_var.split(':') {
                path.push(repo.to_owned());
            }

            // add installed packages directory
            path.push(sys_db.to_string_lossy().to_string());

            // remove duplicates and empty entries from paths
            let mut set: HashSet<String> = HashSet::new();
            path.retain(|x| !x.is_empty() && set.insert(x.clone()));

            path
        };

        Config {
            choice: true,
            debug: false,
            force: false,
            prompt: true,
            strip: true,
            lvl: 1,
            pid,
            proc,
            kiss_cache_dir,
            kiss_compress,
            kiss_path,
            kiss_root,
            kiss_tmp_dir,
            mak_dir,
            pkg_dir,
            tar_dir,
            tmp_dir,
            cho_db,
            pkg_db,
            sys_db,
            sources_dir,
            log_dir,
            bin_dir,
        }
    }
}

// define a trait to hold deps
#[derive(Debug, Default, Clone)]
pub struct Dependencies {
    pub normal: Vec<String>,
    pub explicit: Vec<String>,
}

impl Dependencies {
    pub fn new() -> Self {
        Dependencies {
            normal: Vec::new(),
            explicit: Vec::new(),
        }
    }
}

// FLAG_CONTEXT management
pub static FLAG_CONTEXT: Lazy<Arc<RwLock<Config>>> =
    Lazy::new(|| Arc::new(RwLock::new(Config::new())));

// Dependencies management
pub static DEPENDENCIES: Lazy<Arc<RwLock<Dependencies>>> =
    Lazy::new(|| Arc::new(RwLock::new(Dependencies::new())));

pub fn get_config() -> RwLockReadGuard<'static, Config> {
    FLAG_CONTEXT.read().unwrap()
}

pub fn set_config(c: &Context, handle_signals: bool) {
    let mut context: RwLockWriteGuard<'_, Config> = FLAG_CONTEXT.write().unwrap();

    #[cfg(feature = "threading")]
    {
        if let Ok(jobs) = c.int_flag("jobs") {
            rayon::ThreadPoolBuilder::new()
                .num_threads(jobs as usize)
                .build_global()
                .expect("Failed to build thread pool");
        }
    }

    // setup signal handling
    if handle_signals {
        unsafe {
            signal(SIGINT, handle_sigint as usize);
            signal(SIGTERM, handle_sigint as usize);
        }
    }

    // bool flags
    context.choice = !c.bool_flag("choice");
    context.debug = c.bool_flag("debug");
    context.force = c.bool_flag("force");
    context.prompt = !c.bool_flag("prompt");
    context.strip = !c.bool_flag("strip");

    if let Ok(pid) = c.int_flag("pid") {
        context.pid = pid as u32;
    }

    if let Ok(kiss_compress) = c.string_flag("kiss-compress") {
        context.kiss_compress = kiss_compress;
    }

    if let Ok(kiss_root) = c.string_flag("kiss-root") {
        context.kiss_root = PathBuf::from(kiss_root);
    }
    // build/cache stuff
    if let Ok(kiss_cache_dir) = c.string_flag("kiss-cache-dir") {
        context.kiss_cache_dir = PathBuf::from(kiss_cache_dir);
    }
    if let Ok(kiss_tmp_dir) = c.string_flag("kiss-tmp-dir") {
        context.kiss_tmp_dir = PathBuf::from(kiss_tmp_dir);
    }
    // db stuff
    if let Ok(cho_db) = c.string_flag("cho-db") {
        context.cho_db = cho_db;
    }
    if let Ok(pkg_db) = c.string_flag("pkg-db") {
        context.pkg_db = pkg_db;
    }
    if let Ok(kiss_path) = c.string_flag("kiss-path") {
        let kiss_path: Vec<String> = {
            let mut path: Vec<String> = Vec::new();

            for repo in kiss_path.split(':') {
                path.push(repo.to_owned());
            }

            // add installed packages directory
            path.push(context.sys_db.to_string_lossy().to_string());

            // remove duplicates and empty entries from paths
            let mut set: HashSet<String> = HashSet::new();
            path.retain(|x| !x.is_empty() && set.insert(x.clone()));

            path
        };

        context.kiss_path = kiss_path;
    }
}

// REPO_NAME and REPO_DIR management
pub static REPO_DIR: Lazy<RwLock<String>> = Lazy::new(|| RwLock::new(get_current_working_dir()));
pub static REPO_NAME: Lazy<RwLock<String>> = Lazy::new(|| {
    let repo_dir = get_repo_dir();
    let result = if Path::new(&repo_dir).exists() {
        get_directory_name(&repo_dir).to_owned()
    } else {
        String::new()
    };
    RwLock::new(result)
});

pub fn set_repo_name(new_value: String) {
    let mut repo_name = REPO_NAME.write().unwrap();
    *repo_name = new_value;
}

pub fn get_repo_name() -> String {
    let repo_name = REPO_NAME.read().unwrap();
    repo_name.clone()
}

pub fn set_repo_dir(new_value: String) {
    let mut repo_dir = REPO_DIR.write().unwrap();
    *repo_dir = new_value;
}

pub fn get_repo_dir() -> String {
    let repo_dir = REPO_DIR.read().unwrap();
    repo_dir.clone()
}
