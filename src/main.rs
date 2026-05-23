use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

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

        // Parse shell input
        let parts = parse_input(input);

        if parts.is_empty() {
            continue;
        }

        let command = &parts[0];
        let args = &parts[1..];

        match command.as_str() {
            // Builtin: exit
            "exit" => {
                break;
            }

            // Builtin: echo
            "echo" => {
                println!("{}", args.join(" "));
            }

            // Builtin: pwd
            "pwd" => match env::current_dir() {
                Ok(path) => {
                    println!("{}", path.display());
                }

                Err(_) => {
                    println!("pwd: unable to get current directory");
                }
            },

            // Builtin: cd
            "cd" => {
                if args.is_empty() {
                    continue;
                }

                let target_dir = if args[0] == "~" {
                    env::var("HOME").unwrap_or_default()
                } else {
                    args[0].clone()
                };

                let path = Path::new(&target_dir);

                match env::set_current_dir(path) {
                    Ok(_) => {}

                    Err(_) => {
                        println!("cd: {}: No such file or directory", args[0]);
                    }
                }
            }

            // Builtin: type
            "type" => {
                if args.is_empty() {
                    println!("type: missing argument");
                    continue;
                }

                let cmd = &args[0];

                match cmd.as_str() {
                    "echo" | "exit" | "type" | "pwd" | "cd" => {
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
                    #[cfg(unix)]
                    {
                        let result = Command::new(&path).arg0(command).args(args).spawn();

                        match result {
                            Ok(mut child) => {
                                child.wait().unwrap();
                            }

                            Err(_) => {
                                println!("{}: command not found", command);
                            }
                        }
                    }

                    #[cfg(windows)]
                    {
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
                }

                None => {
                    println!("{}: command not found", command);
                }
            },
        }
    }
}

// Parse shell input
fn parse_input(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();

    let mut in_single_quotes = false;
    let mut in_double_quotes = false;

    let chars: Vec<char> = input.chars().collect();

    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        match ch {
            // Backslash outside quotes
            '\\' if !in_single_quotes && !in_double_quotes => {
                i += 1;

                if i < chars.len() {
                    current.push(chars[i]);
                }
            }

            // Single quotes
            '\'' if !in_double_quotes => {
                in_single_quotes = !in_single_quotes;
            }

            // Double quotes
            '"' if !in_single_quotes => {
                in_double_quotes = !in_double_quotes;
            }

            // Argument split outside quotes
            ' ' | '\t' if !in_single_quotes && !in_double_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }

            // Normal characters
            _ => {
                current.push(ch);
            }
        }

        i += 1;
    }

    // Push last argument
    if !current.is_empty() {
        args.push(current);
    }

    args
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

// Check executable permissions
fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        if let Ok(metadata) = fs::metadata(path) {
            let mode = metadata.permissions().mode();

            return mode & 0o111 != 0;
        }
    }

    #[cfg(windows)]
    {
        return path.is_file();
    }

    false
}
