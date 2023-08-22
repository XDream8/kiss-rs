use seahorse::{App, Context, Flag, FlagType};
use shared_lib::globals::{get_config, set_config, Config};
use std::env;
use std::process::exit;
use std::sync::RwLockReadGuard;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

fn main() {
    // cli
    let args: Vec<String> = env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!(
            "{} <replacement> <package>",
            env!("CARGO_PKG_NAME")
        ))
        .action(provides_action);

    app.run(args);
}

fn provides_action(c: &Context) {
    set_config(c, false);
    let config: RwLockReadGuard<'_, Config> = get_config();

    if c.args.is_empty() {
        if let Err(err) = list_provides(&config.provides_db) {
            eprintln!("ERROR: {}", err);
            exit(1);
        };
    } else if c.args.len() <= 2 {
        let replacement: Option<&str> = if c.args.len() == 1 {
            None
        } else {
            Some(c.args[0].as_str())
        };
        let replaces: &str = if c.args.len() == 1 {
            c.args[0].as_str()
        } else {
            c.args[1].as_str()
        };

        if let Err(err) = add_remove_from_provides(&config.provides_db, replacement, replaces) {
            eprintln!("ERROR: {}", err);
            exit(1);
        }
    } else {
        eprintln!(
            "ERROR: {} does not accept more than 2 args",
            env!("CARGO_PKG_NAME")
        );
        exit(1);
    }
}

fn list_provides(provides_path: &Path) -> Result<(), io::Error> {
    let file: File = File::open(provides_path)?;
    let reader: BufReader<File> = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;

        let parts: Vec<&str> = line.split_whitespace().collect();

        println!("{} -> {}", parts[0], parts[1]);
    }

    Ok(())
}

fn add_remove_from_provides(
    provides_path: &Path,
    replacement: Option<&str>,
    replaces: &str,
) -> Result<(), io::Error> {
    // Read the file into memory
    let mut lines: Vec<String> = Vec::new();
    if let Ok(file) = File::open(provides_path) {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            lines.push(line?);
        }
    }

    let provides_line: String = if let Some(replacement) = replacement {
        format!("{} {}", replacement, replaces)
    } else {
        format!("{}", replaces)
    };

    // Check if the desired line exists
    if replacement.is_some()
        && !lines
            .iter()
            .any(|line| line == &provides_line || line.ends_with(&provides_line))
    {
        // add it if it does not exists
        println!("adding {} -> {}", replacement.unwrap_or(""), replaces);
        lines.push(provides_line.to_string());
    } else if replacement.is_none() {
        // remove it if it already exists
        println!("removing {}", provides_line);
        lines.retain(|x| !x.starts_with(replaces));
    } else if replacement.is_some() {
        println!("removing {} -> {}", replacement.unwrap_or(""), replaces);
        lines.retain(|x| x != &provides_line);
    }

    // Sort the lines
    lines.sort();

    // Write the updated and sorted lines back to the file
    let mut file = File::create(provides_path)?;
    for line in lines {
        writeln!(file, "{}", line)?;
    }

    Ok(())
}
