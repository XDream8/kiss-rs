mod install;

// for cli-args
use seahorse::{App, Context};
use shared_lib::flags::*;
use shared_lib::get_args;
use shared_lib::globals::{get_config, set_config, Config};
use std::env;
use std::process::exit;
use std::sync::RwLockReadGuard;

use shared_lib::signal::pkg_clean;
use shared_lib::{get_current_working_dir, get_directory_name};

use self::install::pkg_install;

fn main() {
    // cli
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [flags] <packages>", env!("CARGO_PKG_NAME")))
        .flag(choice_flag())
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
        .action(action);

    app.run(args);
    // Handle exit signal
    exit(pkg_clean(0));
}

fn action(c: &Context) {
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
