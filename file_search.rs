use std::fs;
use std::io::{self, Write};
use std::path::Path;

fn main() {
    // Prompt the user for the search string
    let search_string = prompt_for_search_string();

    // Search for files recursively starting from the current directory
    let current_dir = Path::new(".");
    println!("Searching for files containing '{}'", search_string);
    search_files(current_dir, &search_string);

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
