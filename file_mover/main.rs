use rayon::prelude::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn move_file(source: &Path, destination: &Path) -> io::Result<()> {
    fs::copy(source, destination).and_then(|_| fs::remove_file(source))
}

fn get_files_in_directory(directory: &str, recursive: bool) -> io::Result<Vec<PathBuf>> {
    let dir_path = Path::new(directory);
    let dir_entry_iter = fs::read_dir(dir_path)?;

    let mut file_paths = vec![];
    for dir_entry in dir_entry_iter {
        let path = dir_entry?.path();
        if path.is_dir() && recursive {
            file_paths.extend(get_files_in_directory(path.to_str().unwrap(), recursive)?);
        } else {
            file_paths.push(path);
        }
    }
    Ok(file_paths)
}

fn main() {
    let file_type_input = r#"Enter the file type you want to move (e.g., *.txt, *.jpg): "#;
    let source_dir_input = "Enter the source directory: ";
    let destination_dir_input = "Enter the destination directory: ";
    let recursive_input = "Do you want to recursively copy all files and subdirectories? (y/n): ";
    let operation_input = "Do you want to copy or cut files? (copy/cut): ";

    println!("{}", file_type_input);
    let mut file_type = String::new();
    io::stdin().read_line(&mut file_type).expect("Failed to read file type");
    let file_type = file_type.replace("*.", "");
    let file_type = file_type.trim();

    println!("{}", source_dir_input);
    let mut source_dir = String::new();
    io::stdin().read_line(&mut source_dir).expect("Failed to read source directory");
    let source_dir = source_dir.trim();

    println!("{}", destination_dir_input);
    let mut destination_dir = String::new();
    io::stdin().read_line(&mut destination_dir).expect("Failed to read destination directory");
    let destination_dir = destination_dir.trim();

    println!("{}", recursive_input);
    let mut recursive = String::new();
    io::stdin().read_line(&mut recursive).expect("Failed to read recursive option");
    let recursive = recursive.trim().to_lowercase() == "y";

    println!("{}", operation_input);
    let mut operation = String::new();
    io::stdin().read_line(&mut operation).expect("Failed to read operation type");
    let operation = operation.trim().to_lowercase() == "copy";

    let file_paths = get_files_in_directory(&source_dir, recursive)
        .expect("Failed to read files from source directory");

    let start = Instant::now();
    let total_files = Arc::new(Mutex::new(0));
    let total_size = Arc::new(Mutex::new(0));

    file_paths.par_iter()
        .filter(|path| path.is_file() && path.extension().map_or(false, |ext| ext.to_str().unwrap_or("") == file_type))
        .for_each(|source_path| {
            let file_name = source_path.file_name().unwrap();
            let destination_path = Path::new(&destination_dir).join(file_name);
            let metadata = fs::metadata(&source_path).unwrap();

            println!("Processing file: {:?}", source_path);

            if operation {
                match fs::copy(&source_path, &destination_path) {
                    Ok(_) => {
                        println!("Copied file to: {:?}", destination_path);
                        *total_files.lock().unwrap() += 1;
                        *total_size.lock().unwrap() += metadata.len();
                    }
                    Err(e) => println!("Failed to copy file: {:?}, Error: {:?}", source_path, e),
                }
            } else {
                match move_file(&source_path, &destination_path) {
                    Ok(_) => {
                        println!("Moved file to: {:?}", destination_path);
                        *total_files.lock().unwrap() += 1;
                        *total_size.lock().unwrap() += metadata.len();
                    }
                    Err(e) => println!("Failed to move file: {:?}, Error: {:?}", source_path, e),
                }
            }
        });

    let duration = start.elapsed();
    println!("Operation completed successfully!");
    println!(
        "Total files transferred: {}, total size transferred: {} bytes, total time elapsed: {:.2?}",
        *total_files.lock().unwrap(),
        *total_size.lock().unwrap(),
        duration
    );
}
