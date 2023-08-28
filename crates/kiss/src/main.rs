use std::env;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

fn main() {
    // Get the path to the current executable
    let exe_path: PathBuf = env::current_exe().unwrap_or_else(|err| {
        eprintln!("Failed to get current executable path: {}", err);
        exit(1);
    });

    // Get the directory containing the executable
    let exe_dir: &Path = exe_path.parent().unwrap_or_else(|| {
        eprintln!("Failed to get executable directory");
        exit(1);
    });

    // Collect command line arguments
    let args: Vec<String> = env::args().collect();

    // Get the binary name from the first argument
    let binary_name: &String = args.get(1).unwrap_or_else(|| {
        eprintln!("No binary name provided");
        exit(1);
    });

    // Construct the binary path in the same directory as the wrapper
    let binary_path_in_exe_dir: PathBuf = exe_dir.join(format!("kiss-{}", binary_name));

    let binary_path: Option<PathBuf> = if binary_path_in_exe_dir.exists() {
        Some(binary_path_in_exe_dir)
    } else {
        // If the binary is not found in the same directory as the wrapper, search the system PATH
        env::var("PATH")
            .expect("Failed to get PATH environment variable")
            .split(':')
            .find_map(|path| {
                let path: String = path.to_string();
                let binary_name: String = format!("kiss-{}", binary_name);
                let binary_path: PathBuf = Path::new(&path).join(&binary_name);
                if binary_path.exists() {
                    Some(binary_path)
                } else if Path::new(&path).exists() {
                    let entries: Vec<_> = match read_dir(path) {
                        Ok(entries) => entries.collect(),
                        Err(err) => {
                            eprintln!("Failed to read directory: {}", err);
                            exit(1);
                        }
                    };

                    entries.iter().find_map(|entry| {
                        let entry = entry.as_ref().unwrap();
                        let path: PathBuf = entry.path();

                        if let Some(file_name) = path.file_name() {
                            let file_name: String = file_name.to_string_lossy().into_owned();

                            if file_name.starts_with(&binary_name) {
                                Some(path)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            })
    };

    // execute command
    if let Some(binary_path) = binary_path {
        // Build the command to execute the binary
        let mut command: Command = Command::new(binary_path);
        // Pass all arguments except the first two (wrapper and binary name)
        command.args(&args[2..]);

        // Execute the binary
        let status = command.status();
        if let Err(err) = status {
            eprintln!("Failed to execute {}: {}", binary_name, err);
            exit(1);
        }
        exit(status.unwrap().code().unwrap_or(1));
    }

    // If the binary is not found, print an error
    eprintln!("No matching binary found in the current directory or PATH");
    exit(1);
}
