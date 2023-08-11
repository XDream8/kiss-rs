use seahorse::{App, Context, Flag, FlagType};
use shared_lib::get_args;
use shared_lib::globals::{get_config, set_config, Config};
use shared_lib::jobs_flag;
use std::env;
use std::sync::RwLockReadGuard;

use shared_lib::{cat, read_a_dir_and_sort};
use std::path::Path;

use shared_lib::die;
use shared_lib::signal::pkg_clean;

// threading
#[cfg(feature = "threading")]
use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use shared_lib::{iter, sort};

fn main() {
    // cli
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [flags] <packages>", env!("CARGO_PKG_NAME")))
        .flag(
            Flag::new("version", FlagType::Bool)
                .description("add version parameter to end of the package queries")
                .alias("v"),
        )
        .flag(jobs_flag!())
        .action(list_action);

    app.run(args);
}

pub fn list_action(c: &Context) {
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
