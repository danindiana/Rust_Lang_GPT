use std::env;
use std::io::Write;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::process::{Command, Stdio};
use std::fs::File;

fn create_pipe() -> (RawFd, RawFd) {
    let mut fds = [0; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } == -1 {
        panic!("Failed to create pipe");
    }
    (fds[0], fds[1])
}

fn main() {
    let (read_fd, write_fd) = create_pipe();

    let child = Command::new("child")
        .arg(read_fd.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn child process");

    unsafe {
        libc::close(read_fd);
    }

    let mut write_pipe = unsafe { File::from_raw_fd(write_fd) };
    let message = "Hello from parent";
    write_pipe.write_all(message.as_bytes()).expect("Failed to write to pipe");

    println!("Parent: Written message to pipe");

    // Close the write end of the pipe to signal EOF to the child process
    drop(write_pipe);

    // Wait for the child process to exit
    let _ = child.wait();
}
