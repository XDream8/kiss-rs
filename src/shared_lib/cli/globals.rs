use seahorse::Context;

use crate::shared_lib::get_env_variable;
use std::collections::HashSet;
use std::path::PathBuf;

use once_cell::sync::Lazy;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

// signal handling
use crate::shared_lib::signal::{create_tmp_dirs, handle_sigint};
use nix::sys::signal::{signal, SigHandler, SIGINT, SIGTERM};

// for http client
// use std::time::Duration;
// use ureq::{Agent, AgentBuilder};

// define a trait to hold deps
#[derive(Debug, Default, Clone)]
pub struct Dependencies {
    pub normal: Vec<String>,
    pub explicit: Vec<String>,
}

// define a struct to hold shared variables
#[derive(Debug, Default, Clone)]
pub struct Config {
    pub choice: bool,
    pub debug: bool,
    pub force: bool,
    pub prompt: bool,
    pub strip: bool,
    pub quiet: bool,
    pub verbose: bool,
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
    pub db: String,
    pub cho_db: String,
    pub pkg_db: String,
    pub sys_db: PathBuf,
    pub provides_db: PathBuf,
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
        let db: String = String::from("var/db/kiss");
        let provides_file_path: String = format!("{}/provides", db);
        let provides_db: PathBuf = kiss_root.join(provides_file_path);
        let cho_db: String = format!("{}/choices", db);
        let pkg_db: String = format!("{}/installed", db);
        let sys_db: PathBuf = kiss_root.join(&pkg_db);

        // and lastly kiss path
        let kiss_path: Vec<String> = {
            let env_var: String = get_env_variable("KISS_PATH", String::new());

            let mut path: Vec<String> = Vec::new();

            for repo in env_var.split(':') {
                path.push(repo.to_owned());
            }

            // remove duplicates and empty entries from paths
            let mut set: HashSet<String> = HashSet::new();
            path.retain(|x| !x.is_empty() && set.insert(x.to_string()));

            path
        };

        Config {
            choice: true,
            debug: false,
            force: false,
            prompt: true,
            strip: true,
            quiet: false,
            verbose: false,
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
            sources_dir,
            log_dir,
            bin_dir,
            db,
            cho_db,
            pkg_db,
            sys_db,
            provides_db,
        }
    }
}

// FLAG_CONTEXT management
pub static FLAG_CONTEXT: Lazy<Arc<RwLock<Config>>> =
    Lazy::new(|| Arc::new(RwLock::new(Config::new())));

// Dependencies management
pub static DEPENDENCIES: Lazy<Arc<RwLock<Dependencies>>> =
    Lazy::new(|| Arc::new(RwLock::new(Dependencies::default())));

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

    // bool flags
    context.choice = !c.bool_flag("choice");
    context.debug = c.bool_flag("debug");
    context.force = c.bool_flag("force");
    context.prompt = !c.bool_flag("prompt");
    context.strip = !c.bool_flag("strip");
    context.quiet = c.bool_flag("quiet");
    context.verbose = c.bool_flag("verbose");

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

            // remove duplicates and empty entries from paths
            let mut set: HashSet<String> = HashSet::new();
            path.retain(|x| !x.is_empty() && set.insert(x.to_string()));

            path
        };

        context.kiss_path = kiss_path;
    }

    // setup signal handling
    if handle_signals {
        // create tmp dirs
        create_tmp_dirs(&context);
        let handler = SigHandler::Handler(handle_sigint);
        unsafe {
            signal(SIGINT, handler).unwrap();
            signal(SIGTERM, handler).unwrap();
        }
    }
}
