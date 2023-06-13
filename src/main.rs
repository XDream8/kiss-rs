// for cli-args
use seahorse::{App, Context, Command};
use std::env;
use std::process::exit;

use rayon::prelude::*;

use kiss::list::list_action;
use kiss::search::search_action;
use kiss::checksum::checksum_action;

use blake3::{Hasher, OutputReader};


fn main() {
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [file(s)] [args]", env!("CARGO_PKG_NAME")))
        .action(action)
        .command(Command::new("list")
		 .description("List installed packages")
		 .alias("l")
		 .usage("kiss list <package>")
		 .action(list_action)
	)
	.command(Command::new("search")
		 .description("Search for packages")
		 .alias("s")
		 .usage("kiss search <package>")
		 .action(search_action)
	)
	.command(Command::new("checksum")
		 .description("Generate checksums")
		 .alias("c")
		 .usage("kiss checksum")
		 .action(checksum_action)
	);

    app.run(args);
}

fn action(c: &Context) {

    match kiss::get_file_hash("/var/db/kiss/installed/pigz/version") {
	Ok(hash) => println!("File hash: {}", hash),
	Err(e) => eprintln!("Error: {}", e)
}

    if c.args.is_empty() {
	c.help();
	exit(0);
    }
}
