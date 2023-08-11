use search_lib::pkg_find_version;
use shared_lib::globals::Config;

use std::fs;
use std::path::Path;

pub fn pkg_cache(config: &Config, pkg: &str) -> Option<String> {
    let version: String = pkg_find_version(config, pkg, false);

    let file: String = format!(
        "{}/{}@{}.tar.",
        config.bin_dir.to_string_lossy(),
        pkg,
        version
    );
    let file_with_ext: String = format!("{}{}", file, config.kiss_compress);

    if Path::new(file_with_ext.as_str()).exists() {
        return Some(file_with_ext);
    } else {
        for entry in fs::read_dir(&config.bin_dir).expect("Failed to read BIN_DIR") {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        if file_name_str.starts_with(file.as_str()) {
                            return Some(file_name_str.to_owned());
                        }
                    }
                }
            }
        }
    }

    None
}
