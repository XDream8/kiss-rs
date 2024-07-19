use clap::{Parser, Subcommand};
use kiss_api::compression::CompressionType;
use nix::unistd::User;
use std::path::PathBuf;

fn str_to_user(name: &str) -> Result<User, String> {
    match User::from_name(name) {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err(String::from("User not found")),
        Err(error) => Err(format!("Error: {}", error)),
    }
}

fn str_to_compressiontype(name: &str) -> Result<CompressionType, String> {
    match name {
        "bz2" => Ok(CompressionType::BZ2),
        "gz" => Ok(CompressionType::GZ),
        "lz4" => Ok(CompressionType::LZ4),
        "xz" => Ok(CompressionType::XZ),
        "zstd" => Ok(CompressionType::ZSTD),
        _ => Err(String::from("Unsupported compression type")),
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Log everything and keep temporary directories.
    #[arg(short, long, env = "KISS_DEBUG")]
    pub debug: bool,

    /// User to use for building packages
    #[arg(long = "user", env = "KISS_BUILDUSER", value_parser = str_to_user)]
    pub build_user: Option<User>,

    /// Compression method to use for built package tarballs.
    #[arg(short, long, default_value = "gz", env = "KISS_COMPRESS", value_parser = str_to_compressiontype)]
    pub compression_type: CompressionType,

    /// Where packages binaries/sources will be at and built.
    #[arg(
        long = "cache",
        default_value = "/var/cache/kiss",
        env = "KISS_CACHEDIR"
    )]
    pub cache_directory: PathBuf,

    /// Where installed packages will go.
    #[arg(short, long, default_value = "/", env = "KISS_ROOT")]
    pub installation_directory: PathBuf,

    /// List of repositories.
    #[arg(short, long, env = "KISS_PATH", value_delimiter = ':')]
    pub repositories: Vec<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List installed packages
    List {
        search_query: Vec<String>,

        /// show versions
        #[arg(short, long)]
        version: bool,
    },
    /// Search packages
    Search {
        search_query: Vec<String>,
        #[arg(short, long)]
        recursive: bool,
        #[arg(short, long)]
        version: bool,
    },
}
