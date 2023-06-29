use std::fs::{self, File};
use std::io::{self, Write, BufWriter};
use std::path::Path;

fn main() {
    // Prompt the user for the search string
    let search_string = prompt_for_search_string();

    // Prompt the user for the directories to scan
    let directories = prompt_for_directories();

    // Ask the user if they want to write the results to a file
    let write_to_file = prompt_to_write_to_file();

    // Collect the results
    let mut results = Vec::new();
    for directory in directories {
        let dir_path = Path::new(&directory);
        if dir_path.is_dir() {
            println!("Searching in directory: {}", directory);
            search_files(dir_path, &search_string, &mut results);
        } else {
            println!("'{}' is not a directory. Skipping...", directory);
        }
    }

    // Output results
    if write_to_file {
        let file_name = format!("my_{}_results.txt", search_string);
        let file = File::create(&file_name).expect("Could not create file");
        let mut writer = BufWriter::new(file);

        for result in results {
            writeln!(writer, "{}", result).expect("Could not write to file");
        }

        println!("Results written to {}", file_name);
    } else {
        for result in results {
            println!("{}", result);
        }
    }
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

fn prompt_to_write_to_file() -> bool {
    loop {
        print!("Do you want to write results to a file? (y/n): ");
        io::stdout().flush().unwrap();

        let mut answer = String::new();
        io::stdin().read_line(&mut answer).unwrap();
        let answer = answer.trim().to_lowercase();

        match answer.as_str() {
            "y" => return true,
            "n" => return false,
            _ => println!("Please enter 'y' or 'n'"),
        }
    }
}

fn search_files(dir: &Path, search_string: &str, results: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    // Recursive call
                    search_files(&path, search_string, results);
                } else if let Some(file_name) = path.file_name() {
                    if file_name.to_string_lossy().contains(search_string) {
                        results.push(format!("Found: {:?}", path));
                    }
                }
            }
        }
    }
}
