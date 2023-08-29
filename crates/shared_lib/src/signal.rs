use crate::globals::{get_config, Config};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::RwLockReadGuard;

pub extern "C" fn handle_sigint(_signal: libc::c_int) {
    println!("Received SIGINT signal");
    process::exit(pkg_clean(0));
}

pub fn create_tmp_dirs(config: &Config) -> i32 {
    let dirs: Vec<&PathBuf> = vec![
        &config.sources_dir,
        &config.log_dir,
        &config.bin_dir,
        &config.mak_dir,
        &config.pkg_dir,
        &config.tar_dir,
        &config.tmp_dir,
    ];
    for dir in dirs {
        if !dir.exists() {
            fs::create_dir_all(dir).expect("Failed to create directory");
        }
    }

    0
}

pub fn pkg_clean(exit_code: i32) -> i32 {
    let config: RwLockReadGuard<'static, Config> = get_config();

    if !config.debug {
        if config.lvl == 1 && config.proc.exists() {
            fs::remove_dir_all::<&Path>(config.proc.as_ref()).expect("Failed to remove directory");
        } else if config.tar_dir.exists() {
            fs::remove_dir_all::<&Path>(config.tar_dir.as_ref())
                .expect("Failed to remove directory");
        }
    }

    exit_code
}
