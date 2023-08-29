use seahorse::{App, Context, Flag, FlagType};
use shared_lib::flags::jobs_flag;
use shared_lib::flags::kiss_path_flag;
use shared_lib::get_args;
use shared_lib::globals::{get_config, set_config, Config};
use std::env;
use std::sync::RwLockReadGuard;

use search_lib::pkg_find;

fn main() {
    // cli
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [flags] <packages>", env!("CARGO_PKG_NAME")))
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
        .action(search_action);

    app.run(args);
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
