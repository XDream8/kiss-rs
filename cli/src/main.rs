use std::path::PathBuf;

use kiss_api::package_info::{pkg_get_info, Package};
use kiss_api::pkg::{pkg_find_and_print, pkg_print_installed_packages};

use kiss_api::error::Error;

use kiss::cli::*;

use clap::Parser;
use kiss_api::source::pkg_download_source;

// will remove this later
#[allow(unused_variables)]

fn create_tmp_dirs(dirs: Vec<&PathBuf>) -> Result<(), Error> {
    dirs.iter().try_for_each(|dir| {
        if !dir.exists() {
            std::fs::create_dir_all(dir).map_err(Error::from)
        } else {
            Ok(())
        }
    })
}

fn clean_tmp_dirs(debug: bool, proc_dir: &PathBuf, tar_dir: &PathBuf) -> Result<(), Error> {
    if !debug {
        if proc_dir.exists() {
            std::fs::remove_dir_all(proc_dir)?
        } else if tar_dir.exists() {
            std::fs::remove_dir_all(tar_dir)?
        }
    }

    Ok(())
}

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

    // create tmp dirs if needed
    match &cli.command {
        Commands::Download { .. } => create_tmp_dirs(vec![
            &source_cache_dir,
            &log_cache_dir,
            &binary_cache_dir,
            &mak_dir,
            &pkg_dir,
            &tar_dir,
            &tmp_dir,
        ])?,
        _ => {}
    }

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

    // clean tmp dirs
    match &cli.command {
        Commands::Download { .. } => clean_tmp_dirs(cli.debug, &proc, &tar_dir)?,
        _ => {}
    }

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
