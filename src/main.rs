use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

fn main() {
    loop {
        // Shell prompt
        print!("$ ");
        io::stdout().flush().unwrap();

        // Read input
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim();

        // Ignore empty input
        if input.is_empty() {
            continue;
        }

        // Split command
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts[0] {
            // Exit command
            "exit" => {
                break;
            }

            // Echo command
            "echo" => {
                let args = parts[1..].join(" ");
                println!("{}", args);
            }

            // Type command
            "type" => {
                // Check argument
                if parts.len() < 2 {
                    println!("type: missing argument");
                    continue;
                }

                let command = parts[1];

                // Builtin commands
                match command {
                    "echo" | "exit" | "type" => {
                        println!("{} is a shell builtin", command);
                    }

                    // Search PATH
                    _ => match find_executable(command) {
                        Some(path) => {
                            println!("{} is {}", command, path);
                        }
                        None => {
                            println!("{}: not found", command);
                        }
                    },
                }
            }

            // Unknown command
            _ => {
                println!("{}: command not found", input);
            }
        }
    }
}

// Function to search executable in PATH
fn find_executable(command: &str) -> Option<String> {
    // Get PATH environment variable
    let path_var = env::var("PATH").unwrap_or_default();

    // Split PATH using OS-specific separator
    for dir in env::split_paths(&path_var) {
        let full_path = dir.join(command);

        // Check if file exists
        if full_path.exists() {
            // Check execute permission
            if is_executable(&full_path) {
                return Some(full_path.to_string_lossy().to_string());
            }
        }
    }

    None
}

// Check execute permission
fn is_executable(path: &Path) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let permissions = metadata.permissions();
            let mode = permissions.mode();

            // Check execute bits
            return mode & 0o111 != 0;
        }

        #[cfg(windows)]
        {
            // Windows: just check file exists
            return metadata.is_file();
        }
    }

    false
}
