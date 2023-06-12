// for cli-args
use seahorse::{App, Context, Command};
use std::env;
use std::process::exit;

use kiss::list::list_action;
use kiss::search::search_action;

fn main() {
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [file(s)] [args]", env!("CARGO_PKG_NAME")))
        .action(action)
        .command(Command::new("list")
		 .description("list packages")
		 .alias("l")
		 .usage("kiss list <package>")
		 .action(list_action)
	)
	.command(Command::new("search")
		 .description("search packages")
		 .alias("s")
		 .usage("kiss search <package>")
		 .action(search_action)
	);

    app.run(args);
}

fn action(c: &Context) {
    if c.args.is_empty() {
        c.help();
        exit(0);
    }
}
