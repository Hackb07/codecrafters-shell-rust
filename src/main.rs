use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
        let mut parts = parse_input(input);

        if parts.is_empty() {
            continue;
        }

        // Redirection targets
        let mut stdout_redirect: Option<String> = None;
        let mut stderr_redirect: Option<String> = None;

        let mut cleaned_parts = Vec::new();

        let mut i = 0;

        while i < parts.len() {
            match parts[i].as_str() {
                ">" | "1>" => {
                    if i + 1 < parts.len() {
                        stdout_redirect = Some(parts[i + 1].clone());

                        i += 2;
                    } else {
                        i += 1;
                    }
                }

                "2>" => {
                    if i + 1 < parts.len() {
                        stderr_redirect = Some(parts[i + 1].clone());

                        i += 2;
                    } else {
                        i += 1;
                    }
                }

                _ => {
                    cleaned_parts.push(parts[i].clone());
                    i += 1;
                }
            }
        }

        parts = cleaned_parts;

        if parts.is_empty() {
            continue;
        }

        let command = &parts[0];
        let args = &parts[1..];

        match command.as_str() {
            // exit
            "exit" => {
                break;
            }

            // echo
            "echo" => {
                let output = format!("{}\n", args.join(" "));

                // Create stderr file even if unused
                if let Some(file_path) = &stderr_redirect {
                    File::create(file_path).unwrap();
                }

                if let Some(file_path) = stdout_redirect {
                    let mut file = File::create(file_path).unwrap();

                    file.write_all(output.as_bytes()).unwrap();
                } else {
                    print!("{}", output);
                }
            }

            // pwd
            "pwd" => {
                let output = match env::current_dir() {
                    Ok(path) => {
                        format!("{}\n", path.display())
                    }

                    Err(_) => "pwd: unable to get current directory\n".to_string(),
                };

                // Create stderr file even if unused
                if let Some(file_path) = &stderr_redirect {
                    File::create(file_path).unwrap();
                }

                if let Some(file_path) = stdout_redirect {
                    let mut file = File::create(file_path).unwrap();

                    file.write_all(output.as_bytes()).unwrap();
                } else {
                    print!("{}", output);
                }
            }

            // cd
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

                let result = env::set_current_dir(path);

                if result.is_err() {
                    let error_output = format!("cd: {}: No such file or directory\n", args[0]);

                    if let Some(file_path) = stderr_redirect {
                        let mut file = File::create(file_path).unwrap();

                        file.write_all(error_output.as_bytes()).unwrap();
                    } else {
                        eprint!("{}", error_output);
                    }
                } else {
                    // Create stderr file if unused
                    if let Some(file_path) = stderr_redirect {
                        File::create(file_path).unwrap();
                    }
                }
            }

            // type
            "type" => {
                if args.is_empty() {
                    let error_output = "type: missing argument\n";

                    if let Some(file_path) = stderr_redirect {
                        let mut file = File::create(file_path).unwrap();

                        file.write_all(error_output.as_bytes()).unwrap();
                    } else {
                        eprint!("{}", error_output);
                    }

                    continue;
                }

                let cmd = &args[0];

                let output = match cmd.as_str() {
                    "echo" | "exit" | "type" | "pwd" | "cd" => {
                        format!("{} is a shell builtin\n", cmd)
                    }

                    _ => match find_executable(cmd) {
                        Some(path) => {
                            format!("{} is {}\n", cmd, path.display())
                        }

                        None => {
                            format!("{}: not found\n", cmd)
                        }
                    },
                };

                // Create stderr file even if unused
                if let Some(file_path) = &stderr_redirect {
                    File::create(file_path).unwrap();
                }

                if let Some(file_path) = stdout_redirect {
                    let mut file = File::create(file_path).unwrap();

                    file.write_all(output.as_bytes()).unwrap();
                } else {
                    print!("{}", output);
                }
            }

            // External commands
            _ => {
                match find_executable(command) {
                    Some(path) => {
                        let mut cmd = Command::new(&path);

                        #[cfg(unix)]
                        {
                            cmd.arg0(command);
                        }

                        cmd.args(args);

                        // Redirect stdout
                        if let Some(file_path) = stdout_redirect {
                            let file = File::create(file_path).unwrap();

                            cmd.stdout(Stdio::from(file));
                        }

                        // Redirect stderr
                        if let Some(file_path) = stderr_redirect {
                            let file = File::create(file_path).unwrap();

                            cmd.stderr(Stdio::from(file));
                        }

                        let result = cmd.spawn();

                        match result {
                            Ok(mut child) => {
                                child.wait().unwrap();
                            }

                            Err(_) => {
                                eprintln!("{}: command not found", command);
                            }
                        }
                    }

                    None => {
                        eprintln!("{}: command not found", command);
                    }
                }
            }
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

        // Backslashes inside double quotes
        if ch == '\\' && in_double_quotes {
            if i + 1 < chars.len() {
                let next = chars[i + 1];

                match next {
                    '"' | '\\' => {
                        current.push(next);
                        i += 2;
                        continue;
                    }

                    _ => {
                        current.push('\\');
                        current.push(next);
                        i += 2;
                        continue;
                    }
                }
            }
        }

        // Redirection operators outside quotes
        if !in_single_quotes && !in_double_quotes {
            // 2>
            if ch == '2' && i + 1 < chars.len() && chars[i + 1] == '>' {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }

                args.push("2>".to_string());

                i += 2;
                continue;
            }

            // 1>
            if ch == '1' && i + 1 < chars.len() && chars[i + 1] == '>' {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }

                args.push("1>".to_string());

                i += 2;
                continue;
            }

            // >
            if ch == '>' {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }

                args.push(">".to_string());

                i += 1;
                continue;
            }
        }

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

            // Split args
            ' ' | '\t' if !in_single_quotes && !in_double_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }

            // Normal chars
            _ => {
                current.push(ch);
            }
        }

        i += 1;
    }

    // Push final arg
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
