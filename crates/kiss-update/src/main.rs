use seahorse::{App, Context};
use shared_lib::globals::{get_config, set_config, Config};
use std::env;
use std::sync::RwLockReadGuard;

use source_lib::{get_repositories, pkg_update_repo};

use shared_lib::log;

fn main() {
    // cli
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [flags] <packages>", env!("CARGO_PKG_NAME")))
        .action(update_action);

    app.run(args);
}

fn update_action(c: &Context) {
    set_config(c, false);
    let config: RwLockReadGuard<'_, Config> = get_config();

    let kiss_path: Vec<String> = config
        .kiss_path
        .iter()
        .cloned()
        .filter(|x| x != &config.sys_db.to_string_lossy().to_string())
        .collect();

    let repositories: Vec<String> = get_repositories(&kiss_path);

    println!("Updating repositories");

    repositories.iter().for_each(|repo_path| {
        log!(repo_path);
        if let Err(err) = pkg_update_repo(repo_path) {
            eprintln!("Error updating repository {}: {}", repo_path, err);
        }
    })
}
