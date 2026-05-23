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

        // Output redirection
        let mut redirect_file: Option<String> = None;

        if let Some(pos) = parts.iter().position(|x| x == ">" || x == "1>") {
            if pos + 1 < parts.len() {
                redirect_file = Some(parts[pos + 1].clone());

                // Remove redirection tokens
                parts.truncate(pos);
            }
        }

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
                let output = format!("{}\n", args.join(" "));

                if let Some(file_path) = redirect_file {
                    let mut file = File::create(file_path).unwrap();
                    file.write_all(output.as_bytes()).unwrap();
                } else {
                    print!("{}", output);
                }
            }

            // Builtin: pwd
            "pwd" => {
                let output = match env::current_dir() {
                    Ok(path) => format!("{}\n", path.display()),
                    Err(_) => "pwd: unable to get current directory\n".to_string(),
                };

                if let Some(file_path) = redirect_file {
                    let mut file = File::create(file_path).unwrap();
                    file.write_all(output.as_bytes()).unwrap();
                } else {
                    print!("{}", output);
                }
            }

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

                if let Some(file_path) = redirect_file {
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
                        #[cfg(unix)]
                        {
                            let mut cmd = Command::new(&path);

                            cmd.arg0(command);
                            cmd.args(args);

                            // Redirect stdout only
                            if let Some(file_path) = redirect_file {
                                let file = File::create(file_path).unwrap();

                                cmd.stdout(Stdio::from(file));
                            }

                            let result = cmd.spawn();

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
                            let mut cmd = Command::new(&path);

                            cmd.args(args);

                            if let Some(file_path) = redirect_file {
                                let file = File::create(file_path).unwrap();

                                cmd.stdout(Stdio::from(file));
                            }

                            let result = cmd.spawn();

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

        // Backslash handling inside double quotes
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

            // Split arguments
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
