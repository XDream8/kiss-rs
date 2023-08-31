use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::Path,
};

pub fn list_provides(provides_path: &Path) -> Result<(), io::Error> {
    let file: File = File::open(provides_path)?;
    let reader: BufReader<File> = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;

        let parts: Vec<&str> = line.split_whitespace().collect();

        println!("{} -> {}", parts[0], parts[1]);
    }

    Ok(())
}

pub fn add_remove_from_provides(
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
        replaces.to_owned()
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
