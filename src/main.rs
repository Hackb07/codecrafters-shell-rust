use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

fn main() {
    loop {
        // Display shell prompt
        print!("$ ");
        io::stdout().flush().unwrap();

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim();

        // Ignore empty input
        if input.is_empty() {
            continue;
        }

        // Split input into command + arguments
        let parts: Vec<&str> = input.split_whitespace().collect();

        let command = parts[0];
        let args = &parts[1..];

        match command {
            // Builtin: exit
            "exit" => {
                break;
            }

            // Builtin: echo
            "echo" => {
                println!("{}", args.join(" "));
            }

            // Builtin: type
            "type" => {
                if args.is_empty() {
                    println!("type: missing argument");
                    continue;
                }

                let cmd = args[0];

                match cmd {
                    "echo" | "exit" | "type" => {
                        println!("{} is a shell builtin", cmd);
                    }

                    _ => match find_executable(cmd) {
                        Some(path) => {
                            println!("{} is {}", cmd, path.display());
                        }

                        None => {
                            println!("{}: not found", cmd);
                        }
                    },
                }
            }

            // External programs
            _ => match find_executable(command) {
                Some(path) => {
                    let result = Command::new(path).args(args).spawn();

                    match result {
                        Ok(mut child) => {
                            child.wait().unwrap();
                        }

                        Err(_) => {
                            println!("{}: command not found", command);
                        }
                    }
                }

                None => {
                    println!("{}: command not found", command);
                }
            },
        }
    }
}

// Search executable in PATH
fn find_executable(command: &str) -> Option<PathBuf> {
    // Read PATH variable
    let path_env = env::var("PATH").unwrap_or_default();

    // Iterate through PATH directories
    for dir in env::split_paths(&path_env) {
        let full_path = dir.join(command);

        // Check if executable exists
        if full_path.is_file() {
            return Some(full_path);
        }
    }

    None
}
