use seahorse::{App, Context};
use shared_lib::flags::*;
use shared_lib::globals::{get_config, set_config, Config, Dependencies, DEPENDENCIES};
use std::env;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use std::path::PathBuf;

use shared_lib::signal::pkg_clean;
use shared_lib::{die, log};

use build_lib::pkg_build_all;
use search_lib::pkg_find_version;
use shared_lib::{prompt, read_a_dir_and_sort};

fn main() {
    // cli
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [flags]", env!("CARGO_PKG_NAME")))
        .flag(choice_flag())
        .flag(debug_flag())
        .flag(force_flag())
        .flag(prompt_flag())
        .flag(quiet_flag())
        .flag(verbose_flag())
        .flag(pid_flag())
        .flag(kiss_compress_flag())
        .flag(kiss_cache_dir_flag())
        .flag(kiss_path_flag())
        .flag(kiss_root_flag())
        .flag(kiss_tmp_dir_flag())
        .flag(jobs_flag())
        .action(upgrade_action);

    app.run(args);
}

fn upgrade_action(c: &Context) {
    set_config(c, true);
    let config: RwLockReadGuard<'_, Config> = get_config();
    let mut dependencies: RwLockWriteGuard<'_, Dependencies> = DEPENDENCIES.write().unwrap();

    pkg_upgrade(&config, &mut dependencies);
}

fn pkg_upgrade(config: &Config, dependencies: &mut Dependencies) {
    log!("Checking for new package versions");

    let installed_packages: Vec<PathBuf> =
        read_a_dir_and_sort(config.sys_db.to_string_lossy().to_string(), false, &[]);

    let packages: Vec<String> = installed_packages
        .iter()
        .filter_map(|path| {
            let path_str: String = path.to_string_lossy().to_string();
            let pkg_name: &str = path_str
                .rsplit('/')
                .next()
                .unwrap_or_else(|| die!("Failed to get package name"));
            let old_ver: String = pkg_find_version(
                config,
                pkg_name,
                Some(&config.sys_db.to_string_lossy().to_string()),
            )
            .unwrap_or_else(|| die!(pkg_name.to_owned() + ":", "Failed to get version"));
            let new_ver: String = pkg_find_version(config, pkg_name, None)
                .unwrap_or_else(|| die!(pkg_name.to_owned() + ":", "Failed to get version"));

            if old_ver != new_ver {
                println!("{pkg_name} {old_ver} => {new_ver}");
                Some(pkg_name.to_owned())
            } else {
                None
            }
        })
        .collect();

    if packages.contains(&String::from("kiss")) {
        log!("Detected package manager update");
        log!("The package manager will be updated first");

        if config.prompt {
            prompt(None);
        }

        log!("Updated the package manager");
        log!("Re-run 'kiss-upgrade' to update your system");
        return;
    }

    if !packages.is_empty() {
        println!(
            "Packages to update ({}): {}",
            packages.len(),
            packages.join(" ").trim_end()
        );
        if config.prompt {
            prompt(None);
        }
        pkg_build_all(config, dependencies, packages);
        log!("Updated all packages");
    } else {
        log!("Nothing to do")
    }
}
