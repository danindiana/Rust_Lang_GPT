use std::io::{self, Read, Write, BufWriter};
use std::fs::File;
use std::sync::{Arc, Mutex};
use glob::glob;
use rayon::prelude::*;
use walkdir::WalkDir;

fn main() -> io::Result<()> {
    println!("Enter the drives to be indexed (separated by comma):");
    let mut drives = String::new();
    io::stdin().read_line(&mut drives)?;

    println!("Enter the type of files to index (use * for all types):");
    let mut file_type = String::new();
    io::stdin().read_line(&mut file_type)?;

    println!("Enter the name of the output index file:");
    let mut output_filename = String::new();
    io::stdin().read_line(&mut output_filename)?;

    let output_filename = output_filename.trim();
    let file_type = file_type.trim();
    let drives: Vec<String> = drives.split(',').map(|s| s.trim().to_string()).collect();

    if output_filename.is_empty() || file_type.is_empty() || drives.is_empty() {
        println!("No output file name, file type or drives provided. Exiting.");
        return Ok(());
    }

    let writer = Arc::new(Mutex::new(BufWriter::new(File::create(output_filename)?)));

    drives.par_iter().for_each(|drive| {
        if let Err(e) = index_drive(&drive, &file_type, &writer) {
            println!("Error while indexing drive {}: {:?}", drive, e);
        }
    });

    println!("Indexing completed. Results written to {}.", output_filename);
    Ok(())
}

fn index_drive<W: Write + Send>(
    drive: &str,
    file_type: &str,
    writer: &Arc<Mutex<W>>,
) -> io::Result<()> {
    println!("Indexing drive: {}", drive);
    for entry in WalkDir::new(drive).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.path().to_string_lossy().ends_with(file_type) {
            let mut writer = writer.lock().unwrap();
            writeln!(writer, "{}", entry.path().display())?;
        }
    }
    Ok(())
}
