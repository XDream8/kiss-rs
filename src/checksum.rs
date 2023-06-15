use seahorse::Context;

use super::file_exists_in_current_dir;
use super::get_args;

use super::source::pkg_source;

pub fn checksum_action(c: &Context) {
    let packages: Vec<&str> = get_args(&c);

    if packages.is_empty() && file_exists_in_current_dir("sources") {
        // let current_dir = format!("{}", get_current_working_dir().expect("Failed to get current working dir").display());

        // let sources: Vec<String> = read_a_files_lines("sources", "ERROR");

        // let directory_name: String = match get_current_directory_name() {
        //     Some(directory_name) => directory_name.to_owned(),
        //     None => {
        // 	eprintln!("Failed to retrieve current directory name");
        // 	exit(1);
        //     }
        // };

        pkg_source(true);

        // for source in sources {
        //     let current_dir = current_dir.clone();

        //     let mut source = source.clone();
        //     let mut dest = String::new();

        //     // consider user-given folder name
        //     if source.contains(" ") {
        // 	let source_parts: Vec<String> = source.split(" ").map(|l| l.to_owned()).collect();
        // 	    source = source_parts.first().unwrap().to_owned();
        // 	    dest = source_parts.last().unwrap().to_owned().trim_end_matches('/').to_owned();
        // 	}

        // 	// test
        // 	let (res, des) = pkg_source_resolve(directory_name.clone(), source.clone(), dest.clone(), true);

        // 	let (res, des) = pkg_source_resolve(directory_name.clone(), source, dest, true);
        //     }

        // match get_file_hash(&(SRC_DIR+"/fuzzel/1.9.1.tar.gz".to_owned())) {
        //     Ok(hash) => println!("File hash: {}", hash),
        //     Err(e) => eprintln!("Error: {}", e)
        // }
    }
}
