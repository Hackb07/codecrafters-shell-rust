use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

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

        // Split command and arguments
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

            // External commands
            _ => match find_executable(command) {
                Some(path) => {
                    let result = Command::new(&path).args(args).spawn();

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

// Find executable in PATH
fn find_executable(command: &str) -> Option<PathBuf> {
    let path_env = env::var("PATH").unwrap_or_default();

    for dir in env::split_paths(&path_env) {
        let full_path = dir.join(command);

        if full_path.is_file() && is_executable(&full_path) {
            return Some(full_path);
        }
    }

    None
}

// Check execute permissions
fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if let Ok(metadata) = fs::metadata(path) {
            let permissions = metadata.permissions();
            let mode = permissions.mode();

            // Any execute bit set
            return mode & 0o111 != 0;
        }
    }

    #[cfg(windows)]
    {
        return path.is_file();
    }

    false
}
