use std::fs;
use std::io::{self, Write};
use std::path::Path;

fn main() {
    // Prompt the user for the search string
    let search_string = prompt_for_search_string();

    // Prompt the user for the directories to scan
    let directories = prompt_for_directories();

    // Search for files in the specified directories
    for directory in directories {
        let dir_path = Path::new(&directory);
        if dir_path.is_dir() {
            println!("Searching in directory: {}", directory);
            search_files(dir_path, &search_string);
        } else {
            println!("'{}' is not a directory. Skipping...", directory);
        }
    }

    // Suggest a file name upon completion
    println!(
        "Suggested file name: my_{}_results.txt",
        search_string
    );
}

fn prompt_for_search_string() -> String {
    print!("Enter the search string: ");
    io::stdout().flush().unwrap(); // flush the output to ensure the prompt appears before the user input

    let mut search_string = String::new();
    io::stdin().read_line(&mut search_string).unwrap();
    search_string.trim().to_string()
}

fn prompt_for_directories() -> Vec<String> {
    let mut directories = Vec::new();

    loop {
        print!("Enter a directory to scan or 'done' to start scanning: ");
        io::stdout().flush().unwrap();

        let mut directory = String::new();
        io::stdin().read_line(&mut directory).unwrap();
        let directory = directory.trim().to_string();

        if directory.to_lowercase() == "done" {
            break;
        }

        directories.push(directory);
    }

    directories
}

fn search_files(dir: &Path, search_string: &str) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    // Recursive call
                    search_files(&path, search_string);
                } else if let Some(file_name) = path.file_name() {
                    if file_name.to_string_lossy().contains(search_string) {
                        println!("Found: {:?}", path);
                    }
                }
            }
        }
    }
}
