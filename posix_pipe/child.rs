use std::env;
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::fs::File;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: child <pipe_fd>");
        return;
    }

    let pipe_fd: i32 = args[1].parse().expect("Invalid pipe_fd");

    let mut read_pipe = unsafe { File::from_raw_fd(pipe_fd) };
    let mut buffer = String::new();
    read_pipe.read_to_string(&mut buffer).expect("Failed to read from pipe");

    println!("Child: Read message from pipe: {}", buffer);
}
