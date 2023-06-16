// for cli-args
use seahorse::{App, Command, Context};
use std::env;
use std::process::exit;

// signal handling
use kiss::{create_tmp_dirs, pkg_clean};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use kiss::checksum::checksum_action;
use kiss::source::download_action;
use kiss::list::list_action;
use kiss::search::search_action;

fn main() {
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [file(s)] [args]", env!("CARGO_PKG_NAME")))
        .action(action)
        .command(
            Command::new("list")
                .description("List installed packages")
                .alias("l")
                .usage("kiss list <package>")
                .action(list_action),
        )
        .command(
            Command::new("search")
                .description("Search for packages")
                .alias("s")
                .usage("kiss search <package>")
                .action(search_action),
        )
        .command(
            Command::new("checksum")
                .description("Generate checksums")
                .alias("c")
                .usage("kiss checksum")
                .action(checksum_action),
        )
	.command(
            Command::new("download")
                .description("Download sources")
                .alias("d")
                .usage("kiss download")
                .action(download_action),
        );

    let interrupted = Arc::new(AtomicBool::new(false));

    let interrupted_clone = Arc::clone(&interrupted);

    // Handle Ctrl-C
    ctrlc::set_handler(move || {
        interrupted_clone.store(true, Ordering::SeqCst);
        println!("Received SIGINT signal");
        process::exit(pkg_clean());
    })
	.expect("Error setting Ctrl-C handler");

    // create tmp dirs
    create_tmp_dirs();
    app.run(args);
    // Handle exit signal
    process::exit(pkg_clean());
}

fn action(c: &Context) {
    if c.args.is_empty() {
        c.help();
        exit(0);
    }
}
