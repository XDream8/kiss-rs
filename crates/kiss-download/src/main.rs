use seahorse::{App, Context};
use shared_lib::flags::*;
use shared_lib::get_args;
use shared_lib::globals::{get_config, set_config, Config};
use std::sync::RwLockReadGuard;
use std::{env, process::exit};

use shared_lib::signal::pkg_clean;

// threading
#[cfg(feature = "threading")]
use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use shared_lib::iter;

use source_lib::pkg_source;

fn main() {
    // cli
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [flags] <packages>", env!("CARGO_PKG_NAME")))
        .flag(debug_flag())
        .flag(pid_flag())
        .flag(kiss_cache_dir_flag())
        .flag(kiss_path_flag())
        .flag(kiss_tmp_dir_flag())
        .flag(jobs_flag())
        .action(download_action);

    app.run(args);
    // Handle exit signal
    exit(pkg_clean(0));
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
