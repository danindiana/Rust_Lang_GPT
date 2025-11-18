use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use glob::glob;

fn main() -> io::Result<()> {
    println!("Enter the file pattern to search (e.g., *.pdf, my_file*.txt):");
    let mut input = String::new();
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;

    let pattern = input.trim();

    if pattern.is_empty() {
        println!("No pattern provided. Exiting.");
        return Ok(());
    }

    let drives = ["C:", "D:", "E:", "F:", "G:", "H:", "I:", "J:"];

    for drive in &drives {
        search_in_directory(&format!("{}\\", drive), &pattern)?;
    }

    Ok(())
}

fn search_in_directory<P: AsRef<Path>>(dir: P, pattern: &str) -> io::Result<()> {
    let dir_path = dir.as_ref();
    if dir_path.is_dir() {
        let search_pattern = format!("{}\\**\\{}", dir_path.display(), pattern);
        for entry in glob(&search_pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => println!("{}", path.display()),
                Err(e) => println!("{:?}", e),
            }
        }
    }
    Ok(())
}
