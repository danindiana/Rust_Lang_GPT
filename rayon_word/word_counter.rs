use std::collections::HashMap;
use std::io::{self, Read, Write, BufReader, BufWriter};
use std::fs::File;
use std::sync::{Mutex, Arc};
use rayon::prelude::*;

fn main() -> io::Result<()> {
    println!("Enter the text file to process:");
    let mut text_filename = String::new();
    io::stdin().read_line(&mut text_filename)?;

    println!("Enter the name of the output file:");
    let mut output_filename = String::new();
    io::stdin().read_line(&mut output_filename)?;

    let text_filename = text_filename.trim();
    let output_filename = output_filename.trim();

    let text = {
        let mut file = BufReader::new(File::open(text_filename)?);
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        contents
    };

    let counter = Arc::new(Mutex::new(HashMap::new()));

    text.par_lines().for_each(|line| {
        let mut counter = counter.lock().unwrap();
        for word in line.split_whitespace() {
            let count = counter.entry(word.to_string()).or_insert(0);
            *count += 1;
        }
    });

    let mut output = BufWriter::new(File::create(output_filename)?);
    for (word, &count) in counter.lock().unwrap().iter() {
        writeln!(output, "{}: {}", word, count)?;
    }

    Ok(())
}
