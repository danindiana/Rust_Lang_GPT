use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write, BufWriter};
use std::path::Path;
use std::sync::{Arc, Mutex};
use glob::glob;
use rayon::prelude::*;
use sha2::{Sha256, Digest};

fn main() -> io::Result<()> {
    println!("Enter the name of the output file:");
    let mut output_filename = String::new();
    io::stdin().read_line(&mut output_filename)?;

    let output_filename = output_filename.trim();

    if output_filename.is_empty() {
        println!("No output file name provided. Exiting.");
        return Ok(());
    }

    let writer = Arc::new(Mutex::new(BufWriter::new(File::create(output_filename)?)));
    let hashes = Arc::new(Mutex::new(HashMap::new()));

    let drives = vec!["C:".to_string(), "D:".to_string(), "E:".to_string(), "F:".to_string(), "G:".to_string(), "H:".to_string(), "I:".to_string(), "J:".to_string()];

    drives.par_iter().for_each(|drive| {
        if let Err(e) = search_in_directory(&format!("{}\\", drive), &hashes, &writer) {
            println!("Error while searching in {}: {:?}", drive, e);
        }
    });

    println!("Search completed. Results written to {}.", output_filename);
    Ok(())
}

fn search_in_directory<W: Write + Send>(
    dir: &str,
    hashes: &Arc<Mutex<HashMap<String, Vec<String>>>>,
    writer: &Arc<Mutex<W>>,
) -> io::Result<()> {
    println!("Searching in directory: {}", dir);

    if Path::new(dir).is_dir() {
        let search_pattern = format!("{}\\**\\*", dir);
        println!("Search pattern: {}", search_pattern);

        for entry in glob(&search_pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    if path.is_file() {
                        let hash = generate_file_hash(&path);

                        let mut hashes = hashes.lock().unwrap();

                        if hashes.contains_key(&hash) {
                            let mut writer = writer.lock().unwrap();
                            writeln!(writer, "Duplicate files (hash: {}):", hash)?;
                            for dup_path in hashes.get(&hash).unwrap() {
                                writeln!(writer, "{}", dup_path)?;
                            }
                            writeln!(writer, "{}", path.display())?;
                        } else {
                            hashes.entry(hash).or_default().push(path.display().to_string());
                        }
                    }
                }
                Err(e) => println!("Error reading path: {:?}", e),
            }
        }
    } else {
        println!("Directory {} does not exist or is not accessible.", dir);
    }
    Ok(())
}

fn generate_file_hash<P: AsRef<Path>>(path: P) -> String {
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => return String::from("Error opening file"),
    };

    let mut sha256 = Sha256::new();
    let mut buffer = [0; 1024];

    loop {
        let bytes_read = match file.read(&mut buffer) {
            Ok(bytes_read) => bytes_read,
            Err(_) => return String::from("Error reading file"),
        };

        if bytes_read == 0 {
            break;
        }

        sha256.update(&buffer[..bytes_read]);
    }

    format!("{:x}", sha256.finalize())
}
