mod build;

// for cli-args
use seahorse::{App, Context};
use std::env;
use std::process::exit;
use shared_lib::get_args;
use shared_lib::globals::{Config, Dependencies, DEPENDENCIES, get_config, set_config};
use shared_lib::flags::*;
use shared_lib::jobs_flag;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use shared_lib::signal::{create_tmp_dirs, pkg_clean};

use self::build::pkg_build_all;

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
        .flag(pid_flag())
        .flag(kiss_compress_flag())
        .flag(kiss_cache_dir_flag())
        .flag(kiss_path_flag())
        .flag(kiss_root_flag())
        .flag(kiss_tmp_dir_flag())
        .flag(jobs_flag!())
        .action(action);

    // create tmp dirs
    create_tmp_dirs();
    app.run(args);
    // Handle exit signal
    exit(pkg_clean(0));
}

fn action(c: &Context) {
    set_config(c, true);
    let config: RwLockReadGuard<'_, Config> = get_config();
    let mut dependencies: RwLockWriteGuard<'_, Dependencies> = DEPENDENCIES.write().unwrap();

    let packages: Vec<&str> = get_args(c);
    pkg_build_all(&config, &mut dependencies, packages);
}
