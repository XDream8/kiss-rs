use seahorse::Context;
use colored::*;

use std::path::Path;
use std::process::exit;

use rayon::prelude::*;

use super::file_exists_in_current_dir;
use super::get_current_working_dir;
use super::get_args;

use super::get_file_hash;
use super::read_a_files_lines;

// global variables
use super::CAC_DIR;
use super::KISS_PID;
use super::PROC;

use std::process;

pub fn checksum_action(c: &Context) {
    let packages: Vec<&str> = get_args(&c);

    if packages.is_empty() && file_exists_in_current_dir("sources") {
	let sources: Vec<String> = read_a_files_lines("sources", "ERROR");

	println!("{}", *CAC_DIR);

	// match get_file_hash(&(SRC_DIR+"/fuzzel/1.9.1.tar.gz".to_owned())) {
	//     Ok(hash) => println!("File hash: {}", hash),
	//     Err(e) => eprintln!("Error: {}", e)
	// }
    }

}
