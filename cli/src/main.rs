use std::path::PathBuf;

use kiss_api::package_info::{pkg_get_info, Package};
use kiss_api::pkg::{pkg_find_and_print, pkg_print_installed_packages};

use kiss_api::error::Error;

use kiss::cli::*;

use clap::Parser;
use kiss_api::source::pkg_download_source;

// will remove this later
#[allow(unused_variables)]

fn handle_command(cli: &Cli) -> Result<(), Error> {
    // Root directory check(this should be as early as possible)
    if !cli.installation_directory.exists() {
        return Err(Error::RootDirNotExists);
    }

    let pid: u32 = std::process::id();

    // Cache(to avoid repeated computations)
    let source_cache_dir: PathBuf = cli.cache_directory.join("sources");
    let log_cache_dir: PathBuf = cli.cache_directory.join("logs");
    let binary_cache_dir: PathBuf = cli.cache_directory.join("bin");

    // tmpdir stuff(to avoid repeated computations)
    let proc: PathBuf = cli
        .cache_directory
        .join("proc")
        .join(pid.to_string().as_str());
    let mak_dir: PathBuf = proc.join("build");
    let pkg_dir: PathBuf = proc.join("pkg");
    let tar_dir: PathBuf = proc.join("extract");
    let tmp_dir: PathBuf = proc.join("tmp");

    // db stuff
    let packages_db_path: String = String::from("var/db/kiss");
    let provides_file_path: String = format!("{}/provides", packages_db_path);
    let provides_db: PathBuf = cli.installation_directory.join(provides_file_path);
    let cho_db_syntax: String = format!("{}/choices", packages_db_path);
    let pkg_db_syntax: String = format!("{}/installed", packages_db_path);
    let sys_package_database: PathBuf = cli.installation_directory.join(pkg_db_syntax);

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Download { download_query } => {
            let packages: Result<Vec<Package>, Error> = download_query
                .iter()
                .map(|query| {
                    pkg_get_info(
                        query,
                        Some(&source_cache_dir),
                        Some(&binary_cache_dir),
                        Some(&cli.compression_type),
                        &cli.repositories,
                    )
                })
                .collect();
            for package in packages? {
                pkg_download_source(&package.name, &package.sources, &tmp_dir)?
            }
        }
        Commands::List {
            search_query,
            version,
        } => pkg_print_installed_packages(search_query.to_vec(), &sys_package_database, *version)?,
        Commands::Search {
            search_query,
            recursive,
            version,
        } => {
            // TODO: get rid of clone
            let mut repositories = cli.repositories.clone();
            repositories.push(sys_package_database);

            for package in search_query {
                pkg_find_and_print(&repositories, package, *recursive, *version)?
            }
        }
    };

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    if let Err(err) = handle_command(&cli) {
        eprintln!("Error: {}", err);
        std::process::exit(0);
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
