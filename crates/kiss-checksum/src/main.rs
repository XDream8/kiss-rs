mod checksum;

// for cli-args
use seahorse::{App, Context};
use std::env;
use shared_lib::get_args;
use shared_lib::globals::{Config, get_config, set_config};
use shared_lib::flags::kiss_path_flag;
use shared_lib::jobs_flag;
use std::sync::RwLockReadGuard;

use crate::checksum::pkg_checksum;

fn main() {
    // cli
    let args: Vec<String> = env::args().collect();

    let app: App = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [flags] <packages>", env!("CARGO_PKG_NAME")))
        .flag(kiss_path_flag())
        .flag(jobs_flag!())
        .action(action);

    app.run(args);
}

pub fn action(c: &Context) {
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
