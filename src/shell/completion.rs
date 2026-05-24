use crate::shell::builtins::BUILTINS;
use crate::shell::utils::is_executable;

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Helper, Context};

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

pub struct ShellCompleter {
    pub last_input: RefCell<String>,
    pub tab_count: RefCell<u8>,
    pub completions: RefCell<HashMap<String, String>>,
}

impl Helper for ShellCompleter {}

impl Hinter for ShellCompleter {
    type Hint = String;
}

impl Highlighter for ShellCompleter {}

impl Validator for ShellCompleter {}

impl ShellCompleter {
    pub fn new() -> Self {
        ShellCompleter {
            last_input: RefCell::new(String::new()),
            tab_count: RefCell::new(0),
            completions: RefCell::new(HashMap::new()),
        }
    }
}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let input = &line[..pos];
        let last_space = input.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let current_arg = &input[last_space..];

        // ======================
        // CUSTOM COMPLETERS
        // ======================

        if input.contains(' ') {
            let words: Vec<&str> = input.split_whitespace().collect();
            if !words.is_empty() {
                let command_name = words[0];
                let completions = self.completions.borrow();
                if let Some(script_path) = completions.get(command_name) {
                    let arg1 = command_name;
                    let arg2 = current_arg;
                    let arg3 = if words.len() >= 2 {
                        words.get(words.len() - 2).unwrap_or(&"")
                    } else {
                        &""
                    };

                    let output = Command::new(script_path)
                        .arg(arg1)
                        .arg(arg2)
                        .arg(arg3)
                        .env("COMP_LINE", input)
                        .env("COMP_POINT", pos.to_string())
                        .output();

                    if let Ok(output) = output {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let mut candidates: Vec<String> = stdout
                            .lines()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();

                        candidates.sort();

                        if candidates.is_empty() {
                            print!("\x07");
                            std::io::stdout().flush().unwrap();
                            return Ok((0, vec![]));
                        }

                        if candidates.len() == 1 {
                            let candidate = &candidates[0];
                            let replacement =
                                format!("{}{} ", &input[..last_space], candidate);
                            return Ok((
                                0,
                                vec![Pair {
                                    display: candidate.clone(),
                                    replacement,
                                }],
                            ));
                        }

                        let lcp = longest_common_prefix(&candidates);
                        if lcp.len() > current_arg.len() {
                            let replacement =
                                format!("{}{}", &input[..last_space], lcp);
                            return Ok((
                                0,
                                vec![Pair {
                                    display: lcp.clone(),
                                    replacement,
                                }],
                            ));
                        }

                        let mut last_input = self.last_input.borrow_mut();
                        let mut tab_count = self.tab_count.borrow_mut();

                        if *last_input == input {
                            *tab_count += 1;
                        } else {
                            *tab_count = 1;
                            *last_input = input.to_string();
                        }

                        if *tab_count == 1 {
                            print!("\x07");
                            std::io::stdout().flush().unwrap();
                            return Ok((0, vec![]));
                        }

                        println!();
                        println!("{}", candidates.join("  "));
                        print!("$ {}", input);
                        std::io::stdout().flush().unwrap();
                        *tab_count = 0;
                        return Ok((0, vec![]));
                    }
                }
            }
        }

        // ======================
        // NORMAL COMPLETION
        // ======================

        let mut matches: Vec<(String, bool)> = Vec::new();
        let is_command_position = !input.contains(' ');

        if is_command_position {
            for builtin in BUILTINS {
                if builtin.starts_with(current_arg) {
                    matches.push((builtin.to_string(), false));
                }
            }

            let path_env = env::var("PATH").unwrap_or_default();
            for dir in env::split_paths(&path_env) {
                if !dir.exists() {
                    continue;
                }
                let entries = match fs::read_dir(&dir) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                for entry in entries {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    let path = entry.path();
                    if path.is_file() && is_executable(&path) {
                        if let Some(name) = path.file_name() {
                            let name = name.to_string_lossy().to_string();
                            if name.starts_with(current_arg) {
                                matches.push((name, false));
                            }
                        }
                    }
                }
            }
        }

        // FILE / DIR COMPLETION
        let (search_dir, prefix) = if let Some(idx) = current_arg.rfind('/') {
            (&current_arg[..idx + 1], &current_arg[idx + 1..])
        } else {
            ("", current_arg)
        };

        let dir_path = if search_dir.is_empty() {
            PathBuf::from(".")
        } else {
            PathBuf::from(search_dir)
        };

        if let Ok(entries) = fs::read_dir(&dir_path) {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    let name = name.to_string_lossy().to_string();
                    if name.starts_with(prefix) {
                        let full_name = format!("{}{}", search_dir, name);
                        matches.push((full_name, path.is_dir()));
                    }
                }
            }
        }

        matches.sort_by(|a, b| a.0.cmp(&b.0));
        matches.dedup_by(|a, b| a.0 == b.0);

        if matches.is_empty() {
            print!("\x07");
            std::io::stdout().flush().unwrap();
            return Ok((0, vec![]));
        }

        if matches.len() == 1 {
            let (completion, is_dir) = matches[0].clone();
            let replacement = if is_dir {
                format!("{}{}/", &input[..last_space], completion)
            } else {
                format!("{}{} ", &input[..last_space], completion)
            };
            return Ok((
                0,
                vec![Pair {
                    display: completion.clone(),
                    replacement,
                }],
            ));
        }

        let names: Vec<String> = matches.iter().map(|m| m.0.clone()).collect();
        let lcp = longest_common_prefix(&names);

        if lcp.len() > current_arg.len() {
            let replacement = format!("{}{}", &input[..last_space], lcp);
            return Ok((
                0,
                vec![Pair {
                    display: lcp.clone(),
                    replacement,
                }],
            ));
        }

        let mut last_input = self.last_input.borrow_mut();
        let mut tab_count = self.tab_count.borrow_mut();

        if *last_input == input {
            *tab_count += 1;
        } else {
            *tab_count = 1;
            *last_input = input.to_string();
        }

        if *tab_count == 1 {
            print!("\x07");
            std::io::stdout().flush().unwrap();
            return Ok((0, vec![]));
        }

        println!();
        let display_matches: Vec<String> = matches
            .iter()
            .map(|(name, is_dir)| {
                if *is_dir {
                    format!("{}/", name)
                } else {
                    name.clone()
                }
            })
            .collect();
        println!("{}", display_matches.join("  "));
        print!("$ {}", input);
        std::io::stdout().flush().unwrap();
        *tab_count = 0;

        Ok((0, vec![]))
    }
}

pub fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }

    let mut prefix = strings[0].clone();
    for s in strings.iter().skip(1) {
        while !s.starts_with(&prefix) {
            prefix.pop();
            if prefix.is_empty() {
                break;
            }
        }
    }
    prefix
}
