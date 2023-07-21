use std::fs;
use std::io;
use std::path::Path;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use walkdir::WalkDir;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json;
use std::fs::File;
use std::io::{Write, BufWriter};
use std::path::PathBuf;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Getting drives...");
    let drives = get_drives()?;
    println!("Drives: {:?}", drives);
    list_drives(&drives);
    let selected_drive = prompt_for_drive().ok_or(io::Error::new(io::ErrorKind::Other, "No drive selected"))?;
    println!("Selected drive: {:?}", selected_drive);
    let file_types = prompt_for_file_types().ok_or(io::Error::new(io::ErrorKind::Other, "No file type selected"))?;
    println!("File types: {:?}", file_types);
    println!("Building index...");
    let path = PathBuf::from(selected_drive);
    let index = build_index(&path, &file_types)?;
    println!("Index built. Compressing and storing...");
    compress_and_store_index(&index)?;
    println!("Done!");
    Ok(())
}

fn prompt_for_file_types() -> Option<Vec<String>> {
    let mut rl = Editor::<()>::new();
    let mut file_types = Vec::new();

    loop {
        match rl.readline("Please enter a file type to index (or 'DONE' to finish): ") {
            Ok(line) => {
                let line = line.trim().to_lowercase();
                if line == "done" {
                    break;
                } else {
                    file_types.push(line);
                }
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C pressed. Exiting.");
                return None;
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D pressed. Exiting.");
                return None;
            },
            Err(err) => {
                println!("Error: {:?}", err);
                return None;
            },
        }
    }

    Some(file_types)
}

fn get_drives() -> io::Result<Vec<String>> {
    let mut drives = Vec::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:", char::from(letter));
        if Path::new(&drive).exists() {
            drives.push(drive);
        }
    }
    Ok(drives)
}

fn list_drives(drives: &[String]) {
    println!("Available drives:");
    for drive in drives {
        println!("{}", drive);
    }
}

fn prompt_for_drive() -> Option<String> {
    let mut rl = Editor::<()>::new();

    loop {
        match rl.readline("Please enter the drive to index (e.g., 'C:'): ") {
            Ok(line) => {
                let mut drive = line.trim().to_string();
                if !drive.ends_with(":") && !drive.ends_with(":/") {
                    drive.push_str(":/");
                } else if drive.ends_with(":") {
                    drive.push('/');
                }
                return Some(drive);
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C pressed. Exiting.");
                return None;
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D pressed. Exiting.");
                return None;
            },
            Err(err) => {
                println!("Error: {:?}", err);
                return None;
            },
        }
    }
}

pub fn build_index(base_path: &Path, file_types: &[String]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut index = Vec::new();

    for entry in WalkDir::new(base_path).into_iter() {
        match entry {
            Ok(entry) => {
                println!("Checking: {:?}", entry.path());

                if !entry.file_type().is_file() {
                    continue;
                }

                if let Some(extension) = get_extension(entry.path()) {
                    let extension = extension.to_lowercase();
                    println!("File extension: {:?}", extension);

                    if file_types.contains(&extension) {
                        println!("Adding to index: {:?}", entry.path());
                        index.push(entry.path().to_path_buf());
                    }
                }
            },
            Err(err) => eprintln!("Error accessing file: {}", err),
        }
    }

    let index_file_path = base_path.join("index.json");
    let index_data = serde_json::to_string(&index)?;
    fs::write(index_file_path, index_data)?;

    Ok(index)
}





fn get_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
}

fn compress_and_store_index(index: &[PathBuf]) -> io::Result<()> {
    let file = File::create("index.json.gz")?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut writer = BufWriter::new(encoder);

    let serialized_index = serde_json::to_string(&index)?;

    writer.write_all(serialized_index.as_bytes())?;

    Ok(())
}
