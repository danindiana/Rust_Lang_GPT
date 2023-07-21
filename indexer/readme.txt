Let's create a Rust file indexer for windows operating systems. The program should be capable of handling special characters and wildcard strings such as *.pdf which should index all .pdf files, etc. The program should build a compressed index of all file types it indexes. The program should scan the computer for local drives, list those drives, and prompt the user for the drive(s) to be indexed and the file type or types to be indexed. While the programming is executing it should output it's operations in the command line. 

ChatGPT
Creating a Rust file indexer for Windows that handles special characters, wildcard strings, and builds a compressed index is quite a complex task and will involve quite a bit of code. Here's a simplified outline of how you might begin to implement it.

The program will need to:

1. Scan the computer for local drives.
2. List those drives.
3. Prompt the user for the drive(s) to be indexed.
4. Prompt the user for the file type(s) to be indexed.
5. Traverse the selected drives and index the specified file types.
6. Compress and store the index.

Here is a rough skeleton for the program in Rust. 

This is just a skeleton, and you'll need to implement the details of each function:

```rust
use std::fs;
use std::io;
use std::env;
use std::path::Path;

fn main() -> io::Result<()> {
    let drives = get_drives()?;
    list_drives(&drives);
    let selected_drive = prompt_for_drive();
    let file_types = prompt_for_file_types();
    let index = build_index(&selected_drive, &file_types)?;
    compress_and_store_index(&index)?;
    Ok(())
}

fn get_drives() -> io::Result<Vec<String>> {
    // TODO: Implement this function.
    // You can use the 'dirs' crate to get the home directory, then list the parent directories.
    Ok(vec![])
}

fn list_drives(drives: &[String]) {
    // TODO: Implement this function.
    // You can print each drive to the console.
}

fn prompt_for_drive() -> String {
    // TODO: Implement this function.
    // You can use the 'rustyline' crate for readline-like functionality.
    String::new()
}

fn prompt_for_file_types() -> Vec<String> {
    // TODO: Implement this function.
    // You can use the 'rustyline' crate for readline-like functionality.
    vec![]
}

fn build_index(selected_drive: &str, file_types: &[String]) -> io::Result<Vec<String>> {
    // TODO: Implement this function.
    // You can use the 'walkdir' crate to traverse the directory.
    // Then, filter by the specified file types and build the index.
    Ok(vec![])
}

fn compress_and_store_index(index: &[String]) -> io::Result<()> {
    // TODO: Implement this function.
    // You can use the 'flate2' crate for compression, and 'bincode' or 'serde_json' for serialization.
    Ok(())
}
```

This program doesn't handle wildcard strings, but you could add this feature by interpreting the file types as regular expressions, and filtering the files based on whether their type matches the regex. 

If you want to support special characters in the file types, you'll need to make sure to handle them correctly in your regexes. You may also need to handle them in your file IO code, especially if you need to support special characters in filenames or file paths.

Remember to include the necessary dependencies in your `Cargo.toml`:

```toml
[dependencies]
walkdir = "2.3.1"
flate2 = "1.0.20"
serde_json = "1.0.64"
rustyline = "8.0.0"
dirs = "3.0.2"
```


To implement the `get_drives()` function, you can use the `dirs` crate to get the home directory, then list the parent directories. Here's an implementation of the function:

```rust
use std::fs;
use std::io;

fn get_drives() -> io::Result<Vec<String>> {
    let home_dir = dirs::home_dir().ok_or(io::Error::new(io::ErrorKind::NotFound, "Home directory not found."))?;
    let parent_dirs = home_dir.parent().ok_or(io::Error::new(io::ErrorKind::NotFound, "Parent directory not found."))?;

    let drives: Vec<String> = fs::read_dir(parent_dirs)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("Failed to read parent directory: {}", err)))?
        .filter_map(|entry| {
            entry.ok()
                .and_then(|e| e.path().to_str().map(String::from))
        })
        .collect();

    Ok(drives)
}
```

In this implementation, we use the `dirs::home_dir()` function to get the home directory. Then, we obtain the parent directory using the `.parent()` method. We read the parent directory using `fs::read_dir(parent_dirs)` and iterate over the directory entries using `filter_map` to convert each entry into a `String` containing the path of the entry. Finally, we collect these paths into a vector and return it as the list of drives.

Make sure to add the `dirs` crate to your `Cargo.toml` file to use the `dirs::home_dir()` function. Add the following line to your `Cargo.toml` under the `[dependencies]` section:

```toml
[dependencies]
dirs = "3.0"
```

This should complete the implementation of the `get_drives()` function. Keep in mind that this implementation assumes that the parent directory of the home directory contains the drives you want to list. You may need to adjust it based on your specific use case or platform.

This version of the program prints its operations to the console by default, since all the print statements in Rust write to the console. If you want to log operations to a file instead, you could use the 'log' and 'simplelog' crates.

Remember that this is only a skeleton of your program, and you'll need to fill out the actual functionality yourself. There are several complex parts of this program, and you'll need to carefully design and test each one to ensure it works correctly.

Will that code work for Windows Operating systems?

Apologies for the oversight. The code provided in the previous response will not work correctly on Windows operating systems. The reason is that the `dirs::home_dir()` function from the `dirs` crate returns the home directory for Unix-like systems (Linux, macOS), but it may not behave as expected on Windows.

To get the list of drives on Windows, you can use the `winapi` and `kernel32` crates, which provide access to Windows API functions. Here's an updated implementation of the `get_drives()` function that works on Windows:

```rust
use std::fs;
use std::io;
use std::ptr;
use std::ffi::OsString;
use std::os::windows::prelude::*;
use winapi::um::fileapi::GetLogicalDriveStringsW;
use winapi::um::winbase::DRIVE_NO_ROOT_DIR;

fn get_drives() -> io::Result<Vec<String>> {
    let buffer_size: u32 = 255;
    let mut buffer: Vec<u16> = Vec::with_capacity(buffer_size as usize);
    let chars_copied = unsafe {
        GetLogicalDriveStringsW(buffer_size, buffer.as_mut_ptr())
    };

    if chars_copied == 0 {
        return Err(io::Error::last_os_error());
    }

    buffer.set_len(chars_copied as usize);
    let drives: Vec<String> = buffer
        .encode_wide()
        .filter(|&c| c != 0)
        .collect::<Vec<u16>>()
        .chunks(4)
        .filter_map(|chunk| {
            let drive = OsString::from_wide(chunk).into_string().ok()?;
            // Exclude drives like "C:\", "D:\", etc. and return only the drive letter
            if !drive.contains(':') || drive.ends_with(':') {
                None
            } else {
                Some(drive)
            }
        })
        .collect();

    Ok(drives)
}
```

This implementation uses the `winapi` crate to call the Windows API function `GetLogicalDriveStringsW`, which returns a null-separated string containing the names of the logical drives on the system. We then process this string to extract the drive names and return them as a list.

To use the `winapi` crate, add the following lines to your `Cargo.toml` file:

```toml
[dependencies]
winapi = { version = "0.3", features = ["winbase", "minwindef", "winnt", "wincon", "handleapi"] }
```

Please note that using the Windows API directly can be more involved than using cross-platform libraries. If cross-platform compatibility is a priority for your application, consider using a cross-platform library like `platform-info` or `sys-info` instead.

Please implement the fn list_drives(drives: &[String]) {} function for windows operating systems.

In order to retrieve the list of drives in Rust on a Windows operating system, we can make use of the `std::fs::read_dir` function and the `std::path::Path` module.

Given that in Windows, drives are mounted from "A:" to "Z:", we can simply iterate over these letters and check if a drive exists at that mount point.

Here's an implementation of the `get_drives` function:

```rust
use std::fs;
use std::io;
use std::path::Path;

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
```

This function starts by creating an empty vector to store the drives. Then, it iterates over the ASCII values of the uppercase letters A-Z. For each letter, it creates a string in the format "{letter}:", then checks if a path with that name exists. If it does, it adds the drive to the vector.

You can then use the `list_drives` function to print each drive to the console:

```rust
fn list_drives(drives: &[String]) {
    for drive in drives {
        println!("{}", drive);
    }
}
```

And you can test these functions in the `main` function:

```rust
fn main() -> io::Result<()> {
    let drives = get_drives()?;
    list_drives(&drives);
    Ok(())
}
```

When you run this program, it should print the names of all the drives on your computer to the console.

Please implement the fn prompt_for_drive() -> String {} function.

For prompting the user, we can use the `rustyline` crate for readline-like functionality, which is great for user input in a terminal.

Firstly, add `rustyline = "8.0.0"` to your `Cargo.toml` file under `[dependencies]`.

Here's how you could implement the `prompt_for_drive` function:

```rust
use rustyline::error::ReadlineError;
use rustyline::Editor;

fn prompt_for_drive() -> Option<String> {
    let mut rl = Editor::<()>::new();

    loop {
        match rl.readline("Please enter the drive to index: ") {
            Ok(line) => return Some(line),
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
```

Here, we create an instance of `Editor`, which is a line editor with history, syntax highlighting, and more. We then enter a loop where we prompt the user to enter the drive to index. If the user enters a line, we return it. If the user presses CTRL-C or CTRL-D, we print a message and return `None`. If any other error occurs, we print it and return `None`.

This function now returns an `Option<String>` instead of `String`. The `Option` enum has two variants, `Some` and `None`. We use `Some` to indicate that we received a value (the user's input), and `None` to indicate that we didn't receive a value (the user pressed CTRL-C or CTRL-D, or an error occurred).

You will need to update your `main` function and other parts of your code to handle this change. Specifically, you will need to check whether `prompt_for_drive` returned `Some` or `None` before continuing to the next step of your program. If it returned `None`, you should probably exit the program or go back to a previous step.

fn list_drives(drives: &[String]) {

To implement the `list_drives` function that takes a reference to a vector of strings and simply outputs each string (in our case, each drive) to the console, you can use the following implementation:

```rust
fn list_drives(drives: &[String]) {
    println!("Available drives:");
    for drive in drives {
        println!("{}", drive);
    }
}
```

This function starts by printing a heading "Available drives:". Then it iterates over each string in the given slice and prints it to the console. The `println!` macro automatically adds a newline after each drive, so each one will be printed on its own line.

Generate the fn prompt_for_file_types() -> Vec<String> {} fucntion.

To prompt the user for file types, you could implement the `prompt_for_file_types` function using a similar pattern as `prompt_for_drive`. Here's how you could implement it using the `rustyline` crate:

```rust
fn prompt_for_file_types() -> Option<Vec<String>> {
    let mut rl = Editor::<()>::new();
    let mut file_types = Vec::new();

    loop {
        match rl.readline("Please enter a file type to index (or 'DONE' to finish): ") {
            Ok(line) => {
                if line.to_uppercase() == "DONE" {
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
```

In this function, we prompt the user to enter a file type to index. If the user enters "DONE" (case insensitive), we break out of the loop and return the file types entered so far. If the user enters a file type, we add it to our list and continue with the next iteration of the loop.

As before, this function returns an `Option<Vec<String>>` to handle cases where the user presses CTRL-C or CTRL-D, or if an error occurs. You will need to update your `main` function and other parts of your code to handle these cases.

Generate the fn build_index(selected_drive: &str, file_types: &[String]) -> io::Result<Vec<String>> {} function.

For the `build_index` function, we're going to use the `walkdir` crate to recursively walk over directories and list files, and then filter those files by the specified file types.

Let's add `walkdir = "2.3.1"` to the `[dependencies]` section of the `Cargo.toml`.

Here's a simple implementation of the `build_index` function:

```rust
use std::fs;
use std::io;
use std::path::Path;
use walkdir::WalkDir;

fn build_index(selected_drive: &str, file_types: &[String]) -> io::Result<Vec<String>> {
    let mut index = Vec::new();

    for entry in WalkDir::new(selected_drive).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path.extension().and_then(|s| s.to_str());

        if let Some(ext) = extension {
            if file_types.iter().any(|&ft| ft == ext) {
                index.push(path.to_string_lossy().to_string());
            }
        }
    }

    Ok(index)
}
```

This function creates an empty vector to store the index. It then uses the `WalkDir::new` function to create a new directory walker for the selected drive. The `into_iter` function is called to turn the walker into an iterator that yields entries for all files in the directory tree.

The `filter_map(Result::ok)` part takes only `Ok` values from the iterator (effectively ignoring errors like lack of permissions to read a directory).

Next, we check if the entry is a file. If it's not a file (it might be a directory), we skip it and move to the next entry.

Then we get the file's extension as a string slice. If the file has an extension and that extension is in our list of file types to index, we add the file's path to the index. 

The `to_string_lossy` method is used to convert the path to a string, replacing any non-Unicode sequences with U+FFFD REPLACEMENT CHARACTER.

Generate the fn compress_and_store_index(index: &[String]) -> io::Result<()> {} function.

To compress and store the index, we'll use `serde_json` for serialization and `flate2` for compression. You'll need to add these dependencies to your `Cargo.toml`:

```toml
[dependencies]
serde_json = "1.0.64"
flate2 = "1.0.20"
serde = "1.0.126"
```

Here's a simple implementation of the `compress_and_store_index` function:

```rust
use std::fs::File;
use std::io::{Write, BufWriter};
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json;

fn compress_and_store_index(index: &[String]) -> io::Result<()> {
    let file = File::create("index.json.gz")?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut writer = BufWriter::new(encoder);

    let serialized_index = serde_json::to_string(&index)?;

    writer.write_all(serialized_index.as_bytes())?;

    Ok(())
}
```

This function first creates a new file `index.json.gz`. Then, it creates a new Gzip encoder with default compression level. A buffered writer is then created for the encoder.

Next, it serializes the index into a JSON string. It writes this string to the Gzip encoder, compressing it in the process.

The function finishes by returning `Ok(())`, indicating that the operation was successful.

Note that this function writes the compressed index to a file named `index.json.gz` in the current directory. You might want to make this configurable, or at least check whether a file with this name already exists and handle that case appropriately.

Also note that this function uses `serde_json::to_string` to serialize the index, which could potentially use a lot of memory if the index is very large. For a more memory-efficient approach, you could use `serde_json::Serializer` to serialize the index directly to the writer without creating an intermediate string.

Refactor this code so that it is correct:

use std::fs;
use std::io;
use std::env;
use std::path::Path;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use walkdir::WalkDir;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json;
use std::fs::File;
use std::io::{Write, BufWriter};

fn main() -> io::Result<()> {
    let drives = get_drives()?;
    list_drives(&drives);
    let selected_drive = prompt_for_drive();
    let file_types = prompt_for_file_types();
    let index = build_index(&selected_drive, &file_types)?;
    compress_and_store_index(&index)?;
    Ok(())
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
        match rl.readline("Please enter the drive to index: ") {
            Ok(line) => return Some(line),
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

fn prompt_for_file_types() -> Option<Vec<String>> {
    let mut rl = Editor::<()>::new();
    let mut file_types = Vec::new();

    loop {
        match rl.readline("Please enter a file type to index (or 'DONE' to finish): ") {
            Ok(line) => {
                if line.to_uppercase() == "DONE" {
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


fn build_index(selected_drive: &str, file_types: &[String]) -> io::Result<Vec<String>> {
    let mut index = Vec::new();

    for entry in WalkDir::new(selected_drive).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path.extension().and_then(|s| s.to_str());

        if let Some(ext) = extension {
            if file_types.iter().any(|&ft| ft == ext) {
                index.push(path.to_string_lossy().to_string());
            }
        }
    }

    Ok(index)
}

fn compress_and_store_index(index: &[String]) -> io::Result<()> {
    let file = File::create("index.json.gz")?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut writer = BufWriter::new(encoder);

    let serialized_index = serde_json::to_string(&index)?;

    writer.write_all(serialized_index.as_bytes())?;

    Ok(())
}

The main potential issue here is the use of `Option` returned values from `prompt_for_drive` and `prompt_for_file_types` without checking for `None` before usage, which could lead to a `panic` at runtime if the user triggers an exit condition from those prompts (such as CTRL-C or CTRL-D).

Below is the corrected code, where the `main` function uses the `?` operator to return early if `None` is returned by the prompts:

```rust
use std::fs;
use std::io;
use std::env;
use std::path::Path;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use walkdir::WalkDir;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json;
use std::fs::File;
use std::io::{Write, BufWriter};

fn main() -> io::Result<()> {
    let drives = get_drives()?;
    list_drives(&drives);
    let selected_drive = prompt_for_drive().ok_or(io::Error::new(io::ErrorKind::Other, "No drive selected"))?;
    let file_types = prompt_for_file_types().ok_or(io::Error::new(io::ErrorKind::Other, "No file type selected"))?;
    let index = build_index(&selected_drive, &file_types)?;
    compress_and_store_index(&index)?;
    Ok(())
}

// ...remaining function definitions are unchanged...
```

In this modification, `ok_or` is used to convert an `Option` to a `Result` with an appropriate error message. If `None` is returned, the `?` operator in the `main` function will cause it to return early with an `io::Error` indicating that no drive or file type was selected.

Note: This version of the program will exit with an error message if the user doesn't select a drive or file type. Depending on your requirements, you might want to handle these cases differently (for example, by prompting the user again or using default values).

Making the corrections and attempting to compile the corrected code returns this error: error[E0507]: cannot move out of a shared reference
   --> src\main.rs:107:39
    |
107 |             if file_types.iter().any(|&ft| ft == ext) {
    |                                       ^--
    |                                        |
    |                                        data moved here
    |                                        move occurs because `ft` has type `std::string::String`, which does not implement the `Copy` trait
    |
help: consider removing the borrow
    |
107 -             if file_types.iter().any(|&ft| ft == ext) {
107 +             if file_types.iter().any(|ft| ft == ext) {
    |

For more information about this error, try `rustc --explain E0507`.

The Rust compiler is indicating that you're trying to move a value out of a borrowed context. You're trying to destructure and move `ft` out of the `file_types` reference, but `String` is not `Copy`, so you can't do that.

You don't actually need to destructure `ft` here. You can directly compare references, because `==` on `String` and `str` will compare the strings, not the references.

Here's the corrected line of code:

```rust
if file_types.iter().any(|ft| ft.as_str() == ext) {
```

This will compare the `&str` in `file_types` with the `&str` `ext`. 

Alternatively, you could convert `ext` to a `String` and compare the strings directly:

```rust
if file_types.iter().any(|ft| ft == &ext.to_string()) {
```

This will compare the `String` in `file_types` with the `String` representation of `ext`. 

Here's the full corrected function:

```rust
fn build_index(selected_drive: &str, file_types: &[String]) -> io::Result<Vec<String>> {
    let mut index = Vec::new();

    for entry in WalkDir::new(selected_drive).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path.extension().and_then(|s| s.to_str());

        if let Some(ext) = extension {
            if file_types.iter().any(|ft| ft.as_str() == ext) {
                index.push(path.to_string_lossy().to_string());
            }
        }
    }

    Ok(index)
}
```

The rest of your code should remain unchanged.

OK that code compiles but when I run it doesn't seem to do anything. After I list the drive, the file to index, and execute it drops back to the command line. Perhaps we need to add some debugging? Or perhaps make user input more tolerant of syntax?

OK. That code so amended produces the following error: 

error[E0428]: the name `prompt_for_file_types` is defined multiple times
   --> src\main.rs:106:1
    |
29  | fn prompt_for_file_types() -> Option<Vec<String>> {
    | ------------------------------------------------- previous definition of the value `prompt_for_file_types` here
...
106 | fn prompt_for_file_types() -> Option<Vec<String>> {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `prompt_for_file_types` redefined here
    |
    = note: `prompt_for_file_types` must be defined only once in the value namespace of this module

My apologies for the confusion. It seems there was a misunderstanding. You should only have one function named `prompt_for_file_types`. The latter function definition I gave was an improved version that includes user input sanitizing. It was meant to replace the previous `prompt_for_file_types` function in your code. 

Simply, remove the original `prompt_for_file_types` function and replace it with the improved one.

Here's how your new `prompt_for_file_types` function should look:

```rust
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
```

I hope this resolves the misunderstanding. I apologize for the earlier confusion.

OK when I run the code it doesn't build an index. It does produce an index.json file but the file itself is empty.

It sounds like the `build_index` function isn't finding any files that match the types you're entering. 

The issue could be due to how the extensions are checked. In your `build_index` function, the extensions are checked without the leading dot, which is how the `Path` function `extension()` works in Rust. However, in your `prompt_for_file_types` function, users are likely to enter extensions with the dot, as that's the common way to express file types. 

To resolve this issue, you should either prompt the user to enter file types without the dot, or modify the `build_index` function to remove the dot before comparing. 

Here's how you can modify the `build_index` function:

```rust
fn build_index(selected_drive: &str, file_types: &[String]) -> io::Result<Vec<String>> {
    let mut index = Vec::new();

    for entry in WalkDir::new(selected_drive).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path.extension().and_then(|s| s.to_str());

        if let Some(ext) = extension {
            // Remove leading dot from file types for comparison.
            let file_types_without_dot: Vec<_> = file_types
                .iter()
                .map(|ft| ft.trim_start_matches('.'))
                .collect();

            if file_types_without_dot.iter().any(|&ft| ft == ext) {
                index.push(path.to_string_lossy().to_string());
            }
        }
    }

    Ok(index)
}
```

With this modification, `build_index` will remove the leading dot from each file type before comparing it to the extension of the file it's examining. This way, if users enter extensions with a leading dot, `build_index` will still be able to correctly identify files of that type.

Remember, though, this will only work if your program has access to the directories it's trying to index. If it doesn't have access (due to permission restrictions, for example), it might not find any files even if they're there. So please also make sure you're running the program with the necessary permissions.

OK. Let's make sure that the program recursively scans the entire drive when selected by the user. I think I've been entering in the directory-as-drive which has no in this example "pdf" files to scan so it is quitting, however it points to an issue that our code may need to be refactored to implement recursively scanning the entire drive in order to build the index.

OK. I tried running the program with admin privileges and it still drops out having not scanned for anything. 

Hmm, it seems something else might be the problem. 

To debug, let's add more logging to the `build_index` function so we can see what's happening while the program runs. We can log the entries it's checking and any errors it encounters during the walk.

Here's how you could modify `build_index` to add this logging:

```rust
fn build_index(selected_drive: &str, file_types: &[String]) -> io::Result<Vec<String>> {
    let mut index = Vec::new();

    for entry in WalkDir::new(selected_drive).into_iter() {
        match entry {
            Ok(entry) => {
                println!("Checking: {}", entry.path().display());

                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                let extension = path.extension().and_then(|s| s.to_str());

                if let Some(ext) = extension {
                    // Remove leading dot from file types for comparison.
                    let file_types_without_dot: Vec<_> = file_types
                        .iter()
                        .map(|ft| ft.trim_start_matches('.'))
                        .collect();

                    if file_types_without_dot.iter().any(|&ft| ft == ext) {
                        index.push(path.to_string_lossy().to_string());
                    }
                }
            },
            Err(err) => {
                eprintln!("Error: {}", err);
            },
        }
    }

    Ok(index)
}
```

With these modifications, the program will print a line to the console for each entry it checks and any errors it encounters. This should give us more information about why it's not finding any files of the specified types. You can remove these logs once you've resolved the issue.

When running the program with these modifications, please ensure the drive letter you're inputting is correct and that the drive contains some files of the types you're specifying. Remember to include the colon after the drive letter (like 'C:' not just 'C'). Also, the program might take a while to scan a large drive, so please be patient.

OK. The debugging output looks good. It appears to scan the drive but isn't building an index based off the desired file type to index?

Here is a snippet of what the code execution looks like now:

Available drives:
C:
D:
Please enter the drive to index: C:
Selected drive: "C:"
Please enter a file type to index (or 'DONE' to finish): .pdf
Please enter a file type to index (or 'DONE' to finish): DONE
File types: [".pdf"]
Building index...
Checking: C:
Checking: C:.git
Checking: C:.git\config
Checking: C:.git\description
Checking: C:.git\HEAD
Checking: C:.git\hooks
Checking: C:.git\hooks\applypatch-msg.sample
Checking: C:.git\hooks\commit-msg.sample
Checking: C:.git\hooks\fsmonitor-watchman.sample
Checking: C:.git\hooks\post-update.sample
Checking: C:.git\hooks\pre-applypatch.sample
Checking: C:.git\hooks\pre-commit.sample
Checking: C:.git\hooks\pre-merge-commit.sample
Checking: C:.git\hooks\pre-push.sample
Checking: C:.git\hooks\pre-rebase.sample
Checking: C:.git\hooks\pre-receive.sample
Checking: C:.git\hooks\prepare-commit-msg.sample
Checking: C:.git\hooks\push-to-checkout.sample
Checking: C:.git\hooks\update.sample
Checking: C:.git\info
Checking: C:.git\info\exclude
Checking: C:.git\objects
Checking: C:.git\objects\info
Checking: C:.git\objects\pack
Checking: C:.git\refs
Checking: C:.git\refs\heads
Checking: C:.git\refs\tags
Checking: C:.gitignore
Checking: C:.vscode
Checking: C:.vscode\launch.json
Checking: C:Cargo.lock
Checking: C:Cargo.toml
Checking: C:index.json
Checking: C:index.json\index.json
Checking: C:index.json.gz
Checking: C:index2.json
Checking: C:index2.json\index.json
Checking: C:src
Checking: C:src\main.rs
Checking: C:target
Checking: C:target\.rustc_info.json
Checking: C:target\CACHEDIR.TAG
Checking: C:target\debug
Checking: C:target\debug\.cargo-lock
Checking: C:target\debug\.fingerprint
Checking: C:target\debug\.fingerprint\adler-a8685633ca341bfe
Checking: C:target\debug\.fingerprint\adler-a8685633ca341bfe\dep-lib-adler
Checking: C:target\debug\.fingerprint\adler-a8685633ca341bfe\invoked.timestamp
Checking: C:target\debug\.fingerprint\adler-a8685633ca341bfe\lib-adler
Checking: C:target\debug\.fingerprint\adler-a8685633ca341bfe\lib-adler.json
Checking: C:target\debug\.fingerprint\bitflags-56a02a966efa82cc
Checking: C:target\debug\.fingerprint\bitflags-56a02a966efa82cc\run-build-script-build-script-build
Checking: C:target\debug\.fingerprint\bitflags-56a02a966efa82cc\run-build-script-build-script-build.json
Checking: C:target\debug\.fingerprint\bitflags-a17710916136c294
Checking: C:target\debug\.fingerprint\bitflags-a17710916136c294\build-script-build-script-build
Checking: C:target\debug\.fingerprint\bitflags-a17710916136c294\build-script-build-script-build.json
Checking: C:target\debug\.fingerprint\bitflags-a17710916136c294\dep-build-script-build-script-build
Checking: C:target\debug\.fingerprint\bitflags-a17710916136c294\invoked.timestamp
Checking: C:target\debug\.fingerprint\bitflags-fc0c4d81ac1e518c
Checking: C:target\debug\.fingerprint\bitflags-fc0c4d81ac1e518c\dep-lib-bitflags

Based on the log, it does look like it's traversing through the files as expected, but it seems like it doesn't find any PDF files. 

There are a couple of things we could try:

1. Add a log message to print the extension of the files it's checking:

```rust
if let Some(ext) = extension {
    println!("File extension: {}", ext);
    // Rest of your code...
}
```

2. Make sure that the drive `C:` you are entering does actually contain PDF files. If it does not, the index would be empty as the function correctly doesn't find any PDF files.

3. Remember that in Windows file systems, file extensions are not case-sensitive. However, in the code, the comparison is case-sensitive. This means that it won't find files with an extension ".PDF" if you've entered ".pdf" as the file type. To fix this, you could compare the extensions in a case-insensitive way:

```rust
if let Some(ext) = extension {
    if file_types.iter().any(|&ft| ft.eq_ignore_ascii_case(ext)) {
        index.push(path.to_string_lossy().to_string());
    }
}
```

Try these steps and see if you get different results.

OK here is what the code output looks like now:

Please enter a file type to index (or 'DONE' to finish): DONE
File types: [".pdf"]
Building index...
Checking: C:
Checking: C:.git
Checking: C:.git\config
Checking: C:.git\description
Checking: C:.git\HEAD
Checking: C:.git\hooks
Checking: C:.git\hooks\applypatch-msg.sample
File extension: sample
Checking: C:.git\hooks\commit-msg.sample
File extension: sample
Checking: C:.git\hooks\fsmonitor-watchman.sample
File extension: sample
Checking: C:.git\hooks\post-update.sample
File extension: sample
Checking: C:.git\hooks\pre-applypatch.sample
File extension: sample
Checking: C:.git\hooks\pre-commit.sample
File extension: sample
Checking: C:.git\hooks\pre-merge-commit.sample
File extension: sample
Checking: C:.git\hooks\pre-push.sample
File extension: sample
Checking: C:.git\hooks\pre-rebase.sample
File extension: sample
Checking: C:.git\hooks\pre-receive.sample
File extension: sample
Checking: C:.git\hooks\prepare-commit-msg.sample
File extension: sample
Checking: C:.git\hooks\push-to-checkout.sample
File extension: sample
Checking: C:.git\hooks\update.sample
File extension: sample
Checking: C:.git\info
Checking: C:.git\info\exclude
Checking: C:.git\objects
Checking: C:.git\objects\info
Checking: C:.git\objects\pack
Checking: C:.git\refs
Checking: C:.git\refs\heads
Checking: C:.git\refs\tags
Checking: C:.gitignore
Checking: C:.vscode
Checking: C:.vscode\launch.json
File extension: json
Checking: C:Cargo.lock
File extension: lock
Checking: C:Cargo.toml
File extension: toml
Checking: C:index.json
Checking: C:index.json\index.json
File extension: json
Checking: C:index.json.gz
File extension: gz
Checking: C:index2.json
Checking: C:index2.json\index.json
File extension: json
Checking: C:src
Checking: C:src\main.rs
File extension: rs
Checking: C:target
Checking: C:target\.rustc_info.json
File extension: json
Checking: C:target\CACHEDIR.TAG
File extension: TAG
Checking: C:target\debug
Checking: C:target\debug\.cargo-lock
Checking: C:target\debug\.fingerprint
Checking: C:target\debug\.fingerprint\adler-a8685633ca341bfe
Checking: C:target\debug\.fingerprint\adler-a8685633ca341bfe\dep-lib-adler
Checking: C:target\debug\.fingerprint\adler-a8685633ca341bfe\invoked.timestamp
File extension: timestamp
Checking: C:target\debug\.fingerprint\adler-a8685633ca341bfe\lib-adler
Checking: C:target\debug\.fingerprint\adler-a8685633ca341bfe\lib-adler.json
File extension: json

From the logs, it looks like the program is running as expected: it's scanning the files in your C: drive, and is correctly identifying the extensions of the files.

However, the program isn't finding any `.pdf` files. There are several possible reasons for this:

1. There are no `.pdf` files in your C: drive, or the subdirectories you're scanning.
2. The `.pdf` files are in directories that the program doesn't have permission to access.

To troubleshoot further, I suggest the following steps:

1. Double-check that there are indeed `.pdf` files in the directories you're scanning.
2. If there are `.pdf` files, make sure they're in directories that the program has permission to access.
3. You could add more logging messages to the program to help troubleshoot. For example, you could log a message every time it finds a `.pdf` file.
4. If possible, try running the program on a different directory or drive that you know contains `.pdf` files.

It would also be helpful to confirm that the file paths being printed in the logs are what you expect to see. Are these the directories you expect the program to be scanning? Are there any directories it's not scanning that it should be?

It doesn't appear to be scanning all the directories in the C drive for which it has access to. In other words there are many directories it appears not to be scanning.

If the program isn't scanning all directories, this might be due to an issue with how the program handles subdirectories. The `walkdir::WalkDir` object is recursive by default, meaning it should visit all subdirectories of the directory you provide. However, this might not work as expected if the program encounters any errors while attempting to access a directory.

Here are some steps you can take to troubleshoot:

1. Add more logging to the program to help identify which directories it's attempting to visit, and whether it's encountering any errors while doing so.

2. Try running the program on a different directory or drive, to see if the issue is specific to the C: drive.

3. Check the permissions on the directories that aren't being scanned. Is it possible that the program doesn't have permission to access these directories?

4. If the program is crashing or exiting early, it might be due to an unhandled error. Make sure the program is properly handling all possible errors that can occur during file system traversal.

Let's add some code to handle potential errors when iterating over the directories. 

Here's how you can modify the `build_index` function to log any errors:

```rust
pub fn build_index(base_path: &Path, file_types: &[String]) -> Result<(), Box<dyn Error>> {
    let mut index = Index::new();

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        println!("Checking: {:?}", entry.path());

        if !entry.file_type().is_file() {
            continue;
        }

        if let Some(extension) = get_extension(entry.path()) {
            println!("File extension: {:?}", extension);

            if file_types.contains(&extension) {
                println!("Adding to index: {:?}", entry.path());
                index.add(entry.path().to_path_buf());
            }
        }
    }

    let index_file_path = base_path.join("index.json");
    index.save(&index_file_path)?;

    Ok(())
}
```

Now, the program will print a message for each file it's adding to the index. This way, you can see whether the program is finding any files of the desired types, and if so, where those files are located. 

The line `WalkDir::new(base_path).into_iter().filter_map(|e| e.ok())` is updated to use `filter_map(|e| e.ok())` to skip over any Result that contains an error. If there's a problem reading a directory (for example, due to insufficient permissions), the program will log an error message and continue with the next directory.

Modifying the code and running it returns these errors: error[E0405]: cannot find trait `Error` in this scope
   --> src\main.rs:108:83
    |
108 | pub fn build_index(base_path: &Path, file_types: &[String]) -> Result<(), Box<dyn Error>> {
    |                                                                                   ^^^^^ not found in this scope
    |
help: consider importing one of these items
    |
1   + use core::error::Error;
    |
1   + use std::error::Error;
    |

error[E0433]: failed to resolve: use of undeclared type `Index`
   --> src\main.rs:109:21
    |
109 |     let mut index = Index::new();
    |                     ^^^^^ use of undeclared type `Index`
    |
help: consider importing one of these items
    |
1   + use core::ops::Index;
    |
1   + use crate::serde_json::value::Index;
    |
1   + use serde_json::value::Index;
    |
1   + use std::ops::Index;
    |

error[E0425]: cannot find function `get_extension` in this scope
   --> src\main.rs:118:34
    |
118 |         if let Some(extension) = get_extension(entry.path()) {
    |                                  ^^^^^^^^^^^^^ not found in this scope

Some errors have detailed explanations: E0405, E0425, E0433.

I apologize for the confusion. In my previous message, I forgot to define the `get_extension` function and to clarify that `Index` is a hypothetical data structure to hold the indexed files. The actual implementation might be different, such as a `HashMap`, a `Vec`, or a custom struct, depending on your needs.

For the purpose of this example, we can just use a `Vec<PathBuf>` to store the indexed files. Also, we will define `get_extension` as a simple function to extract the extension of a given path. Here is how to correct the code:

First, make sure to import the necessary items:

```rust
use std::path::PathBuf;
use std::error::Error;
```

Then, replace the `build_index` function with this:

```rust
pub fn build_index(base_path: &Path, file_types: &[String]) -> Result<(), Box<dyn Error>> {
    let mut index = Vec::new();

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        println!("Checking: {:?}", entry.path());

        if !entry.file_type().is_file() {
            continue;
        }

        if let Some(extension) = get_extension(entry.path()) {
            println!("File extension: {:?}", extension);

            if file_types.contains(&extension) {
                println!("Adding to index: {:?}", entry.path());
                index.push(entry.path().to_path_buf());
            }
        }
    }

    let index_file_path = base_path.join("index.json");
    let index_data = serde_json::to_string(&index)?;
    fs::write(index_file_path, index_data)?;

    Ok(())
}
```

And add the `get_extension` function:

```rust
fn get_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
}
```

This updated code will store the indexed files as a vector of `PathBuf` and write it to `index.json` in JSON format. The `get_extension` function is used to get the file extension from a `Path`.

Ok modifying the code and running it produces these errors:

warning: unused import: `std::path::PathBuf`
  --> src\main.rs:11:5
   |
11 | use std::path::PathBuf;
   |     ^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` on by default

error[E0308]: mismatched types
   --> src\main.rs:24:29
    |
24  |     let index = build_index(&selected_drive, &file_types)?;
    |                 ----------- ^^^^^^^^^^^^^^^ expected `&Path`, found `&String`
    |                 |
    |                 arguments to this function are incorrect
    |
    = note: expected reference `&Path`
               found reference `&std::string::String`
note: function defined here
   --> src\main.rs:110:8
    |
110 | pub fn build_index(base_path: &Path, file_types: &[String]) -> Result<(), Box<dyn Error>> {
    |        ^^^^^^^^^^^ ----------------

error[E0277]: `?` couldn't convert the error to `std::io::Error`
  --> src\main.rs:24:58
   |
14 | fn main() -> io::Result<()> {
   |              -------------- expected `std::io::Error` because of this
...
24 |     let index = build_index(&selected_drive, &file_types)?;
   |                                                          ^ the trait `From<Box<dyn std::error::Error>>` is not implemented for `std::io::Error`
   |
   = note: the question mark operation (`?`) implicitly performs a conversion on the error value using the `From` trait
   = help: the following other types implement trait `From<T>`:
             <std::io::Error as From<CompressError>>
             <std::io::Error as From<DecompressError>>
             <std::io::Error as From<ErrorKind>>
             <std::io::Error as From<IntoInnerError<W>>>
             <std::io::Error as From<NulError>>
             <std::io::Error as From<serde_json::Error>>
             <std::io::Error as From<walkdir::Error>>
   = note: required for `Result<(), std::io::Error>` to implement `FromResidual<Result<Infallible, Box<dyn std::error::Error>>>`

error[E0308]: mismatched types
   --> src\main.rs:26:30
    |
26  |     compress_and_store_index(&index)?;
    |     ------------------------ ^^^^^^ expected `&[String]`, found `&()`
    |     |
    |     arguments to this function are incorrect
    |
    = note: expected reference `&[std::string::String]`
               found reference `&()`
note: function defined here
   --> src\main.rs:146:4
    |
146 | fn compress_and_store_index(index: &[String]) -> io::Result<()> {
    |    ^^^^^^^^^^^^^^^^^^^^^^^^ ----------------

error[E0433]: failed to resolve: use of undeclared crate or module `fs`
   --> src\main.rs:132:5
    |
132 |     fs::write(index_file_path, index_data)?;
    |     ^^ use of undeclared crate or module `fs`

Some errors have detailed explanations: E0277, E0308, E0433.
For more information about an error, try `rustc --explain E0277`.

The error messages you're getting are saying:

1. You're not using the `PathBuf` import (that's just a warning and it's okay to ignore for now, we'll address this later).
2. The function `build_index` expects a `&Path` but you're providing a `&String`.
3. The `?` operator in `main` is unable to convert a `Box<dyn Error>` into a `std::io::Error`.
4. The function `compress_and_store_index` expects a `&[String]`, but you're providing a `&()`.
5. The `fs` module is not imported.

Here are the corrections you need to make:

1. Convert `selected_drive` into a `PathBuf` before passing it to `build_index`.

2. Change the return type of `main` to `Result<(), Box<dyn Error>>`.

3. Pass `&index` to `compress_and_store_index`.

4. Add the import for `fs`.

Here's the updated code:

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::error::Error;
use std::io;

fn main() -> Result<(), Box<dyn Error>> {
    let selected_drive = get_drive_letter()?;
    let file_types = get_file_types()?;

    let path = PathBuf::from(selected_drive);
    let index = build_index(&path, &file_types)?;

    compress_and_store_index(&index)?;

    Ok(())
}
```

In `build_index` function, return `Vec<PathBuf>` instead of `()`. The updated `build_index` function would look like:

```rust
pub fn build_index(base_path: &Path, file_types: &[String]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut index = Vec::new();

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        println!("Checking: {:?}", entry.path());

        if !entry.file_type().is_file() {
            continue;
        }

        if let Some(extension) = get_extension(entry.path()) {
            println!("File extension: {:?}", extension);

            if file_types.contains(&extension) {
                println!("Adding to index: {:?}", entry.path());
                index.push(entry.path().to_path_buf());
            }
        }
    }

    let index_file_path = base_path.join("index.json");
    let index_data = serde_json::to_string(&index)?;
    fs::write(index_file_path, index_data)?;

    Ok(index)
}
```

Finally, change the argument of `compress_and_store_index` to accept a `&[PathBuf]`:

```rust
fn compress_and_store_index(index: &[PathBuf]) -> io::Result<()> {
    // function body
}
```

These changes should address all of the error messages you received. You may still need to update the `compress_and_store_index` function to handle `PathBuf`s instead of `String`s.

Please implement those changes to this code and reproduce the refactored code in it's entirety: 

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

fn main() -> io::Result<()> {
    println!("Getting drives...");
    let drives = get_drives()?;
    println!("Drives: {:?}", drives);
    list_drives(&drives);
    let selected_drive = prompt_for_drive().ok_or(io::Error::new(io::ErrorKind::Other, "No drive selected"))?;
    println!("Selected drive: {:?}", selected_drive);
    let file_types = prompt_for_file_types().ok_or(io::Error::new(io::ErrorKind::Other, "No file type selected"))?;
    println!("File types: {:?}", file_types);
    println!("Building index...");
    let index = build_index(&selected_drive, &file_types)?;
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
        match rl.readline("Please enter the drive to index: ") {
            Ok(line) => return Some(line),
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


pub fn build_index(base_path: &Path, file_types: &[String]) -> Result<(), Box<dyn Error>> {
    let mut index = Vec::new();

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        println!("Checking: {:?}", entry.path());

        if !entry.file_type().is_file() {
            continue;
        }

        if let Some(extension) = get_extension(entry.path()) {
            println!("File extension: {:?}", extension);

            if file_types.contains(&extension) {
                println!("Adding to index: {:?}", entry.path());
                index.push(entry.path().to_path_buf());
            }
        }
    }

    let index_file_path = base_path.join("index.json");
    let index_data = serde_json::to_string(&index)?;
    fs::write(index_file_path, index_data)?;

    Ok(())
}

fn get_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
}




fn compress_and_store_index(index: &[String]) -> io::Result<()> {
    let file = File::create("index.json.gz")?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut writer = BufWriter::new(encoder);

    let serialized_index = serde_json::to_string(&index)?;

    writer.write_all(serialized_index.as_bytes())?;

    Ok(())
}

Here's your refactored code, based on your original code and the changes discussed above:

```rust
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
        match rl.readline("Please enter the drive to index: ") {
            Ok(line) => return Some(line),
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

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        println!("Checking: {:?}", entry.path());

        if !entry.file_type().is_file() {
            continue;
        }

        if let Some(extension) = get_extension(entry.path()) {
            println!("File extension: {:?}", extension);

            if file_types.contains(&extension) {
                println!("Adding to index: {:?}", entry.path());
                index.push(entry.path().to_path_buf());
            }
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
```

Please note that I've changed the `compress_and_store_index` function to accept a slice of `PathBuf` instead of `String`, and the `main` function's return type is now `Result<(), Box<dyn Error>>`. This change allows it to handle different types of errors, not just IO errors. The call to `build_index` now uses `PathBuf::from(selected_drive)` to convert the string into a path.

Running that code for C drive produces this interesting debug output: 

File extension: "json"
Error: Os { code: 5, kind: PermissionDenied, message: "Access is denied." }
error: process didn't exit successfully: `target\debug\rustindexer.exe` (exit code: 1)

The error message "Access is denied" suggests that the program tried to access a file or directory for which it does not have the necessary permissions. This could be due to the file being locked or the user that the program is running under not having sufficient privileges to access the file.

In this case, it's possible that the program tried to index system files in the C drive, or files currently being used by other applications, to which it does not have access.

You may want to run the program with elevated privileges (e.g., "Run as Administrator" on Windows), but be aware that this can pose a security risk. If the program unintentionally modifies or deletes critical system files, it could cause serious problems.

An alternative approach is to catch the error when you attempt to access a file and continue with the next file. This would look something like this in the `build_index` function:

```rust
pub fn build_index(base_path: &Path, file_types: &[String]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut index = Vec::new();

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        println!("Checking: {:?}", entry.path());

        if !entry.file_type().is_file() {
            continue;
        }

        if let Some(extension) = get_extension(entry.path()) {
            println!("File extension: {:?}", extension);

            if file_types.contains(&extension) {
                println!("Adding to index: {:?}", entry.path());
                match entry.path().to_path_buf() {
                    Ok(path_buf) => index.push(path_buf),
                    Err(e) => eprintln!("Failed to access: {:?} due to {:?}", entry.path(), e)
                }
            }
        }
    }

    let index_file_path = base_path.join("index.json");
    let index_data = serde_json::to_string(&index)?;
    fs::write(index_file_path, index_data)?;

    Ok(index)
}
```

Note that the change in the code is just to demonstrate the approach, but it may not work because `to_path_buf()` method does not return a `Result`, so it doesn't actually throw an error if it fails. You would need to replace it with the actual action that may fail due to the lack of permissions.

This way, even if a file access fails, your program will continue to the next file rather than exiting entirely. It will print a message for each file it failed to access, so you'll know which ones caused problems.

OK lets refactor the following code to implement in such a way that even if a file access fails, the program will continue to the next file rather than stop/exit on error.

pub fn build_index(base_path: &Path, file_types: &[String]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut index = Vec::new();

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        println!("Checking: {:?}", entry.path());

        if !entry.file_type().is_file() {
            continue;
        }

        if let Some(extension) = get_extension(entry.path()) {
            println!("File extension: {:?}", extension);

            if file_types.contains(&extension) {
                println!("Adding to index: {:?}", entry.path());
                index.push(entry.path().to_path_buf());
            }
        }
    }

    let index_file_path = base_path.join("index.json");
    let index_data = serde_json::to_string(&index)?;
    fs::write(index_file_path, index_data)?;

    Ok(index)
}

OK the program is now running but it isn't recursively scanning all directories which contain, for this example, .pdf files? 

`WalkDir::new()` should scan directories recursively by default. If it's not scanning all directories, it might be due to a permissions issue or some files might be inaccessible. 

However, there's another potential issue. If your `file_types` contains ".pdf" (including the dot), the comparison in the code would fail because `get_extension()` returns the extension without the dot. 

Ensure that your `file_types` vector only contains the extensions like "pdf", not ".pdf". 

Here's the updated code snippet:

```rust
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
        .map(|s| s.to_string())
}
```

In this updated code, we're converting the file extension to lower case before comparing it with the `file_types` to ensure the case does not cause mismatches. Also, `get_extension()` now returns the extension as a `String` (not lower case), which matches the type of `file_types`. 

Please check that you're providing the correct inputs and that the directories you're trying to access have the necessary permissions. If you're still having trouble, please provide more details about the directories that are not being scanned.

Modifying the code with that snipper produces the following error: error[E0428]: the name `get_extension` is defined multiple times
   --> src\main.rs:145:1
    |
138 | fn get_extension(path: &Path) -> Option<String> {
    | ----------------------------------------------- previous definition of the value `get_extension` here
...
145 | fn get_extension(path: &Path) -> Option<String> {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `get_extension` redefined here
    |
    = note: `get_extension` must be defined only once in the value namespace of this module

For more information about this error, try `rustc --explain E0428`.

My apologies for the confusion. That error occurred because I inadvertently redefined the `get_extension` function. The `get_extension` function doesn't need to be changed. Here's the corrected version of the `build_index` function:

```rust
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
```

Just use the original `get_extension` function:

```rust
fn get_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
}
```

Now, `build_index` should work as intended: recursively traversing directories and continuing to the next file when it encounters an error accessing a file. The `file_types` check has also been adjusted to ensure a case-insensitive comparison with the file extension.

OK. I figured out what the problem was. WHen the user inputs the "Please enter the drive to index: " for C: in the command line the correct syntax needs to be "C:/" and not "C" or "C:". 

I'm glad you figured out the issue. Yes, the trailing `/` is important to indicate that you're referring to the root directory of the drive and not the drive itself.

To avoid this issue in the future and enhance the user experience, you could modify your `prompt_for_drive` function to automatically append the trailing `/` if it's not provided by the user. Here is a version of the function that does this:

```rust
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
```

This way, no matter whether the user inputs `C`, `C:`, or `C:/`, they will all be treated as `C:/` by the program.
