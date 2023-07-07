use std::fs::File;
use std::io::{self, Read, Write, BufWriter};
use std::path::Path;
use std::sync::{Arc, Mutex};
use glob::glob;
use rayon::prelude::*;
use sha2::{Sha256, Digest};

fn main() -> io::Result<()> {
    println!("Enter the file pattern to search (e.g., *.pdf, my_file*.txt):");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let pattern = input.trim();

    if pattern.is_empty() {
        println!("No pattern provided. Exiting.");
        return Ok(());
    }

    println!("Enter the name of the output file:");
    let mut output_filename = String::new();
    io::stdin().read_line(&mut output_filename)?;

    let output_filename = output_filename.trim();

    if output_filename.is_empty() {
        println!("No output file name provided. Exiting.");
        return Ok(());
    }

    let writer = Arc::new(Mutex::new(BufWriter::new(File::create(output_filename)?)));

    let drives = vec!["C:".to_string(), "D:".to_string(), "E:".to_string(), "F:".to_string(), "G:".to_string(), "H:".to_string(), "I:".to_string(), "J:".to_string()];

    drives.par_iter().for_each(|drive| {
        if let Err(e) = search_in_directory(&format!("{}\\", drive), &pattern, &writer) {
            println!("Error while searching in {}: {:?}", drive, e);
        }
    });

    println!("Search completed. Results written to {}.", output_filename);
    Ok(())
}

fn search_in_directory<W: Write + Send>(dir: &str, pattern: &str, writer: &Arc<Mutex<W>>) -> io::Result<()> {
    println!("Searching in directory: {}", dir);

    if Path::new(dir).is_dir() {
        let search_pattern = format!("{}\\**\\{}", dir, pattern);
        println!("Search pattern: {}", search_pattern);

        for entry in glob(&search_pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    println!("{}", path.display());
                    let hash = generate_file_hash(&path);
                    let mut writer = writer.lock().unwrap();
                    writeln!(writer, "{},{}", path.display(), hash)?;
                },
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
