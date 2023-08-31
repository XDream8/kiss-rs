use crate::shared_lib::{
    globals::{Config, Dependencies},
    prompt, read_a_dir_and_sort,
};

use std::path::PathBuf;

use crate::shared_lib::signal::pkg_clean;
use crate::{die, log};

use crate::build_lib::pkg_build_all;
use crate::search_lib::pkg_find_version;

pub fn pkg_upgrade(config: &Config, dependencies: &mut Dependencies) {
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
