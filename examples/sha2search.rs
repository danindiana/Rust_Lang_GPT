use std::fs::{self, File};
use std::io::{self, Write, BufWriter, Read};
use std::path::{Path, PathBuf};
use glob::glob;
use sha2::{Sha256, Digest};

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

    println!("Enter the name of the output file:");
    let mut output_filename = String::new();
    io::stdout().flush()?;
    io::stdin().read_line(&mut output_filename)?;

    let output_filename = output_filename.trim();

    if output_filename.is_empty() {
        println!("No output file name provided. Exiting.");
        return Ok(());
    }

    println!("Do you want to generate a hash for each file? (yes/no):");
    let mut hash_choice = String::new();
    io::stdout().flush()?;
    io::stdin().read_line(&mut hash_choice)?;

    let generate_hash = hash_choice.trim().eq_ignore_ascii_case("yes");

    let output_file = File::create(output_filename)?;
    let mut writer = BufWriter::new(output_file);

    let drives = ["C:", "D:", "E:", "F:", "G:", "H:", "I:", "J:"];

    for drive in &drives {
        search_in_directory(&format!("{}\\", drive), &pattern, &mut writer, generate_hash)?;
    }

    println!("Search completed. Results written to {}.", output_filename);
    Ok(())
}

fn search_in_directory<P: AsRef<Path>, W: Write>(dir: P, pattern: &str, writer: &mut W, generate_hash: bool) -> io::Result<()> {
    let dir_path = dir.as_ref();
    if dir_path.is_dir() {
        let search_pattern = format!("{}\\**\\{}", dir_path.display(), pattern);
        for entry in glob(&search_pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    println!("{}", path.display());
                    if generate_hash {
                        match generate_file_hash(&path) {
                            Ok(hash) => {
                                println!("Hash: {}", hash);
                                writeln!(writer, "{}\t{}", path.display(), hash)?;
                            }
                            Err(e) => {
                                println!("Failed to generate hash: {:?}", e);
                                writeln!(writer, "{}\tFailed to generate hash", path.display())?;
                            }
                        }
                    } else {
                        writeln!(writer, "{}", path.display())?;
                    }
                },
                Err(e) => println!("{:?}", e),
            }
        }
    }
    Ok(())
}

fn generate_file_hash<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}
