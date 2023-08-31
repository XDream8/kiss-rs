use crate::shared_lib::globals::{get_config, Config};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::RwLockReadGuard;

pub extern "C" fn handle_sigint(_signal: nix::libc::c_int) {
    println!("Received SIGINT signal");
    process::exit(pkg_clean());
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
    dirs.iter().for_each(|dir| {
        if !dir.exists() {
            if let Err(err) = fs::create_dir_all(dir) {
                eprintln!("Failed to create directory: {}", err);
                process::exit(1);
            }
        }
    });

    0
}

pub fn pkg_clean() -> i32 {
    let config: RwLockReadGuard<'static, Config> = get_config();

    if !config.debug {
        if config.lvl == 1 && config.proc.exists() {
            if let Err(err) = fs::remove_dir_all::<&Path>(&config.proc) {
                eprintln!("Failed to remove directory: {}", err);
                return 1;
            }
        } else if config.tar_dir.exists() {
            if let Err(err) = fs::remove_dir_all::<&Path>(&config.tar_dir) {
                eprintln!("Failed to remove directory: {}", err);
                return 1;
            }
        }
    }

    0
}
