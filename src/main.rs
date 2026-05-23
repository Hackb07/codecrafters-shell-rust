use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{
    Cmd, CompletionType, Config, Context, Editor, Helper, KeyCode, KeyEvent, Modifiers,
};

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

// ======================
// BUILTINS
// ======================

const BUILTINS: [&str; 6] = ["echo", "exit", "pwd", "cd", "type", "complete"];

// ======================
// TAB COMPLETER
// ======================

struct ShellCompleter {
    last_input: RefCell<String>,
    tab_count: RefCell<u8>,
    completions: RefCell<HashMap<String, String>>,
}

impl Helper for ShellCompleter {}

impl Hinter for ShellCompleter {
    type Hint = String;
}

impl Highlighter for ShellCompleter {}

impl Validator for ShellCompleter {}

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
        // REGISTERED COMPLETERS
        // ======================

        if input.contains(' ') {
            let words: Vec<&str> = input.split_whitespace().collect();

            if !words.is_empty() {
                let command_name = words[0];

                let completions = self.completions.borrow();

                if let Some(script_path) = completions.get(command_name) {
                    // argv[1]
                    let arg1 = command_name;

                    // argv[2]
                    let arg2 = current_arg;

                    // argv[3]
                    let arg3 = if words.len() >= 2 {
                        words.get(words.len() - 2).unwrap_or(&"")
                    } else {
                        &""
                    };

                    let output = Command::new(script_path)
                        .arg(arg1)
                        .arg(arg2)
                        .arg(arg3)
                        .output();

                    if let Ok(output) = output {
                        let stdout = String::from_utf8_lossy(&output.stdout);

                        let lines: Vec<&str> = stdout.lines().collect();

                        // SINGLE COMPLETION
                        if lines.len() == 1 {
                            let candidate = lines[0].trim();

                            let replacement = format!("{}{} ", &input[..last_space], candidate);

                            return Ok((
                                0,
                                vec![Pair {
                                    display: candidate.to_string(),

                                    replacement,
                                }],
                            ));
                        }
                    }
                }
            }
        }

        let mut matches: Vec<(String, bool)> = Vec::new();

        let is_command_position = !input.contains(' ');

        // ======================
        // COMMAND COMPLETION
        // ======================

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

        // ======================
        // FILE / DIR COMPLETION
        // ======================

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

        // ======================
        // SORT + DEDUP
        // ======================

        matches.sort_by(|a, b| a.0.cmp(&b.0));

        matches.dedup_by(|a, b| a.0 == b.0);

        // ======================
        // NO MATCHES
        // ======================

        if matches.is_empty() {
            print!("\x07");

            std::io::stdout().flush().unwrap();

            return Ok((0, vec![]));
        }

        // ======================
        // SINGLE MATCH
        // ======================

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

        // ======================
        // LONGEST COMMON PREFIX
        // ======================

        let names: Vec<String> = matches.iter().map(|m| m.0.clone()).collect();

        let lcp = longest_common_prefix(&names);

        if lcp.len() > current_arg.len() {
            let replacement = format!("{}{}", &input[..last_space], lcp);

            return Ok((
                0,
                vec![Pair {
                    display: replacement.clone(),

                    replacement,
                }],
            ));
        }

        // ======================
        // MULTIPLE MATCHES
        // ======================

        let mut last_input = self.last_input.borrow_mut();

        let mut tab_count = self.tab_count.borrow_mut();

        if *last_input == input {
            *tab_count += 1;
        } else {
            *tab_count = 1;

            *last_input = input.to_string();
        }

        // FIRST TAB
        if *tab_count == 1 {
            print!("\x07");

            std::io::stdout().flush().unwrap();

            return Ok((0, vec![]));
        }

        // SECOND TAB
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

// ======================
// MAIN
// ======================

fn main() {
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();

    let helper = ShellCompleter {
        last_input: RefCell::new(String::new()),

        tab_count: RefCell::new(0),

        completions: RefCell::new(HashMap::new()),
    };

    let mut rl = Editor::<ShellCompleter, DefaultHistory>::with_config(config).unwrap();

    rl.set_helper(Some(helper));

    rl.bind_sequence(KeyEvent(KeyCode::Tab, Modifiers::NONE), Cmd::Complete);

    loop {
        let readline = rl.readline("$ ");

        match readline {
            Ok(line) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                let mut parts = parse_input(input);

                if parts.is_empty() {
                    continue;
                }

                let mut stdout_redirect: Option<(String, bool)> = None;

                let mut stderr_redirect: Option<(String, bool)> = None;

                let mut cleaned_parts = Vec::new();

                let mut i = 0;

                while i < parts.len() {
                    match parts[i].as_str() {
                        ">" | "1>" => {
                            if i + 1 < parts.len() {
                                stdout_redirect = Some((parts[i + 1].clone(), false));

                                i += 2;
                            } else {
                                i += 1;
                            }
                        }

                        ">>" | "1>>" => {
                            if i + 1 < parts.len() {
                                stdout_redirect = Some((parts[i + 1].clone(), true));

                                i += 2;
                            } else {
                                i += 1;
                            }
                        }

                        "2>" => {
                            if i + 1 < parts.len() {
                                stderr_redirect = Some((parts[i + 1].clone(), false));

                                i += 2;
                            } else {
                                i += 1;
                            }
                        }

                        "2>>" => {
                            if i + 1 < parts.len() {
                                stderr_redirect = Some((parts[i + 1].clone(), true));

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
                    "exit" => {
                        break;
                    }

                    "echo" => {
                        let output = format!("{}\n", args.join(" "));

                        if let Some((file_path, append)) = &stderr_redirect {
                            let _ = open_file(file_path, *append);
                        }

                        if let Some((file_path, append)) = stdout_redirect {
                            let mut file = open_file(&file_path, append);

                            file.write_all(output.as_bytes()).unwrap();
                        } else {
                            print!("{}", output);
                        }
                    }

                    "pwd" => {
                        println!("{}", env::current_dir().unwrap().display());
                    }

                    "cd" => {
                        if args.is_empty() {
                            continue;
                        }

                        let target = if args[0] == "~" {
                            env::var("HOME").unwrap_or_default()
                        } else {
                            args[0].clone()
                        };

                        if let Err(_) = env::set_current_dir(&target) {
                            eprintln!("cd: {}: No such file or directory", target);
                        }
                    }

                    "type" => {
                        if args.is_empty() {
                            continue;
                        }

                        let cmd = &args[0];

                        if BUILTINS.contains(&cmd.as_str()) {
                            println!("{} is a shell builtin", cmd);
                        } else {
                            match find_executable(cmd) {
                                Some(path) => {
                                    println!("{} is {}", cmd, path.display());
                                }

                                None => {
                                    println!("{}: not found", cmd);
                                }
                            }
                        }
                    }

                    "complete" => {
                        // REGISTER
                        if args.len() >= 3 && args[0] == "-C" {
                            let script = args[1].clone();

                            let cmd = args[2].clone();

                            if let Some(helper) = rl.helper_mut() {
                                helper.completions.borrow_mut().insert(cmd, script);
                            }
                        }
                        // PRINT
                        else if args.len() >= 2 && args[0] == "-p" {
                            let cmd = &args[1];

                            if let Some(helper) = rl.helper_mut() {
                                let completions = helper.completions.borrow();

                                match completions.get(cmd) {
                                    Some(path) => {
                                        println!("complete -C '{}' {}", path, cmd);
                                    }

                                    None => {
                                        println!("complete: {}: no completion specification", cmd);
                                    }
                                }
                            }
                        }
                    }

                    _ => match find_executable(command) {
                        Some(path) => {
                            let mut cmd = Command::new(&path);

                            #[cfg(unix)]
                            {
                                cmd.arg0(command);
                            }

                            cmd.args(args);

                            if let Some((file_path, append)) = stdout_redirect {
                                let file = open_file(&file_path, append);

                                cmd.stdout(Stdio::from(file));
                            }

                            if let Some((file_path, append)) = stderr_redirect {
                                let file = open_file(&file_path, append);

                                cmd.stderr(Stdio::from(file));
                            }

                            match cmd.spawn() {
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
                    },
                }
            }

            Err(ReadlineError::Interrupted) => {
                break;
            }

            Err(ReadlineError::Eof) => {
                break;
            }

            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }
}

// ======================
// LCP
// ======================

fn longest_common_prefix(strings: &[String]) -> String {
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

// ======================
// FILE OPEN
// ======================

fn open_file(path: &str, append: bool) -> File {
    OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .unwrap()
}

// ======================
// PARSER
// ======================

fn parse_input(input: &str) -> Vec<String> {
    let mut args = Vec::new();

    let mut current = String::new();

    let mut in_single_quotes = false;
    let mut in_double_quotes = false;

    let chars: Vec<char> = input.chars().collect();

    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

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
            '\\' if !in_single_quotes && !in_double_quotes => {
                i += 1;

                if i < chars.len() {
                    current.push(chars[i]);
                }
            }

            '\'' if !in_double_quotes => {
                in_single_quotes = !in_single_quotes;
            }

            '"' if !in_single_quotes => {
                in_double_quotes = !in_double_quotes;
            }

            ' ' | '\t' if !in_single_quotes && !in_double_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());

                    current.clear();
                }
            }

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

// ======================
// EXECUTABLE SEARCH
// ======================

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

// ======================
// EXECUTABLE CHECK
// ======================

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
