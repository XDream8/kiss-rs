use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

#[inline]
pub fn pkg_get_provides(pkg: &str, provides_path: &Path) -> Result<String, io::Error> {
    let file: File = File::open(provides_path)?;
    let reader: BufReader<File> = BufReader::new(file);

    // find the replacement if there is any
    for line in reader.lines() {
        let line: &String = &line?;
        if line.starts_with('#') {
            continue;
        };
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() == 2 && parts[0] == pkg {
            return Ok(parts[1].to_owned());
        }
    }

    // if we did not find an replacement return pkg
    Ok(pkg.to_owned())
}
