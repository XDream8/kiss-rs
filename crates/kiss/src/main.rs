// for cli-args
use seahorse::{App, Command, Context};
use std::env;
use std::process::exit;

fn main() {
    // first check if we need root privileges
    // Define the actions that require root privileges.
    // let root_actions: [&str; 6] = ["a", "alternatives", "i", "install", "r", "remove"];

    // Get the action from the command-line arguments.
    // let action_arg: String = env::args().nth(1).unwrap_or_default();

    // if root_actions.contains(&action_arg.as_str()) && !am_owner(&KISS_ROOT).unwrap() {
    //     let args: Vec<String> = env::args().collect();
    //     let action_args: Vec<&str> = args.iter().skip(1).map(|arg| arg.as_str()).collect();

    //     run_action_as_root(action_args, false)
    // }

    // cli
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [file(s)] [args]", env!("CARGO_PKG_NAME")))
        .action(action);

    app.run(args);
}

fn action(c: &Context) {
    if c.args.is_empty() {
        c.help();
        exit(0);
    }
}
