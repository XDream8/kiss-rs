use kiss::shared_lib::flags::*;
use kiss::shared_lib::get_args;
use seahorse::{App, Command, Context, Flag, FlagType};
use std::env;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use std::path::Path;
use std::process::exit;

use kiss::build_lib::pkg_build_all;
use kiss::checksum_lib::pkg_checksum;
use kiss::install::pkg_install;
use kiss::provides_lib::{add_remove_from_provides, list_provides};
use kiss::search_lib::pkg_find;
use kiss::shared_lib::{
    cat, get_current_working_dir, get_directory_name,
    globals::{get_config, set_config, Config, Dependencies, DEPENDENCIES},
    log, read_a_dir_and_sort,
};
use kiss::source_lib::{get_repositories, pkg_source, pkg_update_repo};
use kiss::upgrade_lib::pkg_upgrade;

use kiss::die;
use kiss::shared_lib::signal::pkg_clean;

// threading
use kiss::{iter, sort};
#[cfg(feature = "threading")]
use rayon::iter::{IndexedParallelIterator, ParallelIterator};

use nix::unistd::Uid;

fn main() {
    // cli
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [flags] <packages>", env!("CARGO_PKG_NAME")))
        .command(
            Command::new("build")
                .description("Build packages")
                .alias("b")
                .flag(debug_flag())
                .flag(force_flag())
                .flag(prompt_flag())
                .flag(verbose_flag())
                .flag(strip_flag())
                .flag(pid_flag())
                .flag(kiss_compress_flag())
                .flag(kiss_cache_dir_flag())
                .flag(kiss_path_flag())
                .flag(kiss_root_flag())
                .flag(kiss_tmp_dir_flag())
                .flag(jobs_flag())
                .action(build_action),
        )
        .command(
            Command::new("checksum")
                .description("Generate checksums")
                .alias("c")
                .flag(debug_flag())
                .flag(verbose_flag())
                .flag(pid_flag())
                .flag(kiss_cache_dir_flag())
                .flag(kiss_path_flag())
                .flag(kiss_root_flag())
                .flag(kiss_tmp_dir_flag())
                .flag(jobs_flag())
                .action(checksum_action),
        )
        .command(
            Command::new("download")
                .description("Download sources")
                .alias("d")
                .flag(debug_flag())
                .flag(verbose_flag())
                .flag(pid_flag())
                .flag(kiss_cache_dir_flag())
                .flag(kiss_path_flag())
                .flag(kiss_root_flag())
                .flag(kiss_tmp_dir_flag())
                .flag(jobs_flag())
                .action(download_action),
        )
        .command(
            Command::new("install")
                .description("Install packages")
                .alias("i")
                .flag(debug_flag())
                .flag(force_flag())
                .flag(prompt_flag())
                .flag(verbose_flag())
                .flag(strip_flag())
                .flag(pid_flag())
                .flag(kiss_compress_flag())
                .flag(kiss_cache_dir_flag())
                .flag(kiss_path_flag())
                .flag(kiss_root_flag())
                .flag(kiss_tmp_dir_flag())
                .flag(jobs_flag())
                .action(install_action),
        )
        .command(
            Command::new("list")
                .description("List installed packages")
                .alias("l")
                .flag(
                    Flag::new("version", FlagType::Bool)
                        .description("add version parameter to end of the package queries")
                        .alias("v"),
                )
                .flag(kiss_root_flag())
                .flag(jobs_flag())
                .action(list_action),
        )
        .command(
            Command::new("provides")
                .description("add/remove replacements from provides file")
                .alias("p")
                .usage(format!(
                    "{} <replacement> <package>",
                    env!("CARGO_PKG_NAME")
                ))
                .action(provides_action),
        )
        .command(
            Command::new("search")
                .description("Search packages")
                .alias("s")
                .flag(
                    Flag::new("all", FlagType::Bool)
                        .description("enable both recursive and version flags")
                        .alias("rv")
                        .alias("a"),
                )
                .flag(
                    Flag::new("recursive", FlagType::Bool)
                        .description("recursively search packages")
                        .alias("r"),
                )
                .flag(
                    Flag::new("version", FlagType::Bool)
                        .description("add version parameter to end of the search queries")
                        .alias("v"),
                )
                .flag(kiss_path_flag())
                .flag(jobs_flag())
                .action(search_action),
        )
        .command(
            Command::new("upgrade")
                .description("Upgrade the system")
                .alias("U")
                .flag(debug_flag())
                .flag(force_flag())
                .flag(prompt_flag())
                .flag(verbose_flag())
                .flag(strip_flag())
                .flag(pid_flag())
                .flag(kiss_compress_flag())
                .flag(kiss_cache_dir_flag())
                .flag(kiss_path_flag())
                .flag(kiss_root_flag())
                .flag(kiss_tmp_dir_flag())
                .flag(jobs_flag())
                .action(upgrade_action),
        )
        .command(
            Command::new("update")
                .description("Update the repositories")
                .alias("u")
                .flag(kiss_path_flag())
                .flag(jobs_flag())
                .action(update_action),
        );

    app.run(args);
    exit(pkg_clean());
}

fn build_action(c: &Context) {
    // Check if the user is running as root
    if !Uid::effective().is_root() {
        eprintln!("This application must be run as root.");
        exit(1);
    }

    set_config(c, true);
    let config: RwLockReadGuard<'_, Config> = get_config();
    let mut dependencies: RwLockWriteGuard<'_, Dependencies> = DEPENDENCIES.write().unwrap();

    let packages: Vec<&str> = get_args(c);

    pkg_build_all(&config, &mut dependencies, packages);
}

fn checksum_action(c: &Context) {
    set_config(c, false);
    let config: RwLockReadGuard<'_, Config> = get_config();

    let packages: Vec<&str> = get_args(c);

    if !packages.is_empty() {
        for package in packages {
            pkg_checksum(&config, package);
        }
    } else {
        pkg_checksum(&config, "");
    }
}

fn download_action(c: &Context) {
    set_config(c, true);
    let config: RwLockReadGuard<'_, Config> = get_config();
    // get packages
    let packages: Vec<&str> = get_args(c);

    if !packages.is_empty() {
        iter!(packages).enumerate().for_each(|(_, package)| {
            pkg_source(&config, package, false, true);
        });
    } else {
        pkg_source(&config, "", false, true);
    }
}

fn install_action(c: &Context) {
    // Check if the user is running as root
    if !Uid::effective().is_root() {
        eprintln!("This application must be run as root.");
        exit(1);
    }

    set_config(c, true);
    let config: RwLockReadGuard<'_, Config> = get_config();

    let packages: Vec<&str> = get_args(c);

    if !packages.is_empty() {
        for package in packages {
            pkg_install(&config, package).expect("Failed to install package");
        }
    } else {
        let current_dir: String = get_current_working_dir();
        let package: &str = get_directory_name(&current_dir);
        pkg_install(&config, package).expect("Failed to install package");
    }
}

fn list_action(c: &Context) {
    set_config(c, false);

    let version_param: bool = c.bool_flag("version");
    let config: RwLockReadGuard<'_, Config> = get_config();

    // search package in installed list
    let search: Vec<&str> = get_args(c);

    if search.is_empty() {
        // get installed packages
        let mut installed_packages: Vec<_> = iter!(read_a_dir_and_sort(
            &*config.sys_db.to_string_lossy(),
            false,
            &[]
        ))
        .enumerate()
        .map(|(_, package)| {
            let file_name = match package.file_name() {
                Some(file_name) => file_name.to_str().unwrap_or(""),
                None => "",
            };
            if version_param {
                let version: String = cat(&package.join("version"))
                    .unwrap()
                    .replace(' ', "-")
                    .replace('\n', "");
                format!("{} {}", file_name, version)
            } else {
                file_name.to_owned()
            }
        })
        .collect();

        // sort and print
        sort!(installed_packages);
        for package in installed_packages {
            println!("{}", package)
        }
    } else {
        let result: Vec<_> = iter!(search)
            .map(|package| {
                let path: &Path = &Path::new(&config.sys_db).join(package);
                if path.exists() {
                    let file_name = match path.file_name() {
                        Some(file_name) => file_name.to_str().unwrap_or(""),
                        None => "",
                    };
                    if version_param {
                        let version: String = cat(&path.join("version"))
                            .unwrap()
                            .replace(' ', "-")
                            .replace('\n', "");
                        format!("{} {}", file_name, version)
                    } else {
                        file_name.to_owned()
                    }
                } else {
                    format!("{} not found", package)
                }
            })
            .collect();

        // sort and print
        for res in result {
            if res.contains("not found") {
                die!(res);
            } else {
                println!("{}", res);
            }
        }
    }
}

fn provides_action(c: &Context) {
    set_config(c, false);
    let config: RwLockReadGuard<'_, Config> = get_config();

    if c.args.is_empty() {
        if let Err(err) = list_provides(&config.provides_db) {
            eprintln!("ERROR: {}", err);
            exit(1);
        };
    } else if c.args.len() <= 2 {
        let replacement: Option<&str> = if c.args.len() == 1 {
            None
        } else {
            Some(c.args[0].as_str())
        };
        let replaces: &str = if c.args.len() == 1 {
            c.args[0].as_str()
        } else {
            c.args[1].as_str()
        };

        if let Err(err) = add_remove_from_provides(&config.provides_db, replacement, replaces) {
            eprintln!("ERROR: {}", err);
            exit(1);
        }
    } else {
        eprintln!(
            "ERROR: {} does not accept more than 2 args",
            env!("CARGO_PKG_NAME")
        );
        exit(1);
    }
}

fn search_action(c: &Context) {
    set_config(c, false);
    let config: RwLockReadGuard<'_, Config> = get_config();

    let all: bool = c.bool_flag("all");
    let recursive_search: bool = match all {
        true => true,
        false => c.bool_flag("recursive"),
    };
    let version_search: bool = match all {
        true => true,
        false => c.bool_flag("version"),
    };

    let packages: Vec<&str> = get_args(c);

    // search package
    for package in packages {
        pkg_find(&config, package, version_search, recursive_search);
    }
}

fn update_action(c: &Context) {
    set_config(c, false);
    let config: RwLockReadGuard<'_, Config> = get_config();

    let repositories: Vec<String> = get_repositories(&config.kiss_path);

    println!("Updating repositories");

    repositories.iter().for_each(|repo_path| {
        log!(repo_path);
        if let Err(err) = pkg_update_repo(repo_path) {
            eprintln!("Error updating repository {}: {}", repo_path, err);
        }
    })
}

fn upgrade_action(c: &Context) {
    set_config(c, true);
    let config: RwLockReadGuard<'_, Config> = get_config();
    let mut dependencies: RwLockWriteGuard<'_, Dependencies> = DEPENDENCIES.write().unwrap();

    pkg_upgrade(&config, &mut dependencies);
}
