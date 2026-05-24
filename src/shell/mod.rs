pub mod builtins;
pub mod completion;
pub mod job;
pub mod parser;
pub mod utils;

use self::builtins::BUILTINS;
use self::completion::ShellCompleter;
use self::job::{reap_jobs, Job};
use self::parser::{expand_variables, is_valid_identifier, parse_input};
use self::utils::{find_executable, open_file};

use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{Cmd, CompletionType, Config, Editor, KeyCode, KeyEvent, Modifiers};

use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub struct Shell {
    rl: Editor<ShellCompleter, DefaultHistory>,
    jobs: Vec<Job>,
    variables: HashMap<String, String>,
    cmd_history: Vec<String>,
    last_written_count: usize,
}

impl Shell {
    pub fn new() -> Self {
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .build();

        let mut rl =
            Editor::<ShellCompleter, DefaultHistory>::with_config(config).unwrap();
        rl.set_helper(Some(ShellCompleter::new()));
        rl.bind_sequence(KeyEvent(KeyCode::Tab, Modifiers::NONE), Cmd::Complete);

        let mut cmd_history = Vec::new();

        if let Ok(histfile) = env::var("HISTFILE") {
            if let Ok(contents) = fs::read_to_string(&histfile) {
                for line in contents.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        cmd_history.push(trimmed.to_string());
                        let _ = rl.add_history_entry(trimmed);
                    }
                }
            }
        }

        Shell {
            rl,
            jobs: Vec::new(),
            variables: HashMap::new(),
            cmd_history,
            last_written_count: 0,
        }
    }

    pub fn run(&mut self) {
        loop {
            reap_jobs(&mut self.jobs);

            let readline = self.rl.readline("$ ");

            match readline {
                Ok(line) => {
                    let input = line.trim().to_string();
                    if input.is_empty() {
                        continue;
                    }

                    self.cmd_history.push(input.clone());
                    let _ = self.rl.add_history_entry(&input);

                    if !self.handle_command(&input) {
                        break;
                    }
                }
                Err(ReadlineError::Interrupted) => break,
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }

        if let Ok(histfile) = env::var("HISTFILE") {
            let mut content = String::new();
            for entry in &self.cmd_history {
                content.push_str(entry);
                content.push('\n');
            }
            let _ = fs::write(&histfile, content);
        }
    }

    fn handle_command(&mut self, input: &str) -> bool {
        let mut parts = parse_input(input);
        if parts.is_empty() {
            return true;
        }

        // BACKGROUND
        let mut background = false;
        if let Some(last) = parts.last() {
            if last == "&" {
                background = true;
                parts.pop();
            }
        }

        // REDIRECTION
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
            return true;
        }

        // PIPELINE
        if parts.contains(&"|".to_string()) {
            let segments: Vec<&[String]> = parts.split(|p| p == "|").collect();
            let n = segments.len();

            if n >= 2 && segments.iter().all(|s| !s.is_empty()) {
                let all_external =
                    segments.iter().all(|s| !BUILTINS.contains(&s[0].as_str()));

                if all_external {
                    let mut children: Vec<std::process::Child> = Vec::new();
                    let mut error = false;

                    for (i, seg) in segments.iter().enumerate() {
                        let cmd_name = &seg[0];
                        let cmd_args = &seg[1..];

                        match find_executable(cmd_name) {
                            Some(path) => {
                                let mut cmd = Command::new(&path);
                                #[cfg(unix)]
                                {
                                    cmd.arg0(cmd_name);
                                }
                                cmd.args(cmd_args);

                                if i > 0 {
                                    if let Some(stdout) =
                                        children[i - 1].stdout.take()
                                    {
                                        cmd.stdin(Stdio::from(stdout));
                                    }
                                }

                                if i < n - 1 {
                                    cmd.stdout(Stdio::piped());
                                } else if let Some((ref file_path, append)) =
                                    stdout_redirect
                                {
                                    cmd.stdout(Stdio::from(open_file(
                                        file_path, append,
                                    )));
                                }

                                if let Some((ref file_path, append)) =
                                    stderr_redirect
                                {
                                    cmd.stderr(Stdio::from(open_file(
                                        file_path, append,
                                    )));
                                }

                                match cmd.spawn() {
                                    Ok(child) => children.push(child),
                                    Err(_) => {
                                        eprintln!("{}: command not found", cmd_name);
                                        error = true;
                                        break;
                                    }
                                }
                            }
                            None => {
                                eprintln!("{}: command not found", cmd_name);
                                error = true;
                                break;
                            }
                        }
                    }

                    if !error {
                        for child in children.iter_mut().rev() {
                            let _ = child.wait();
                        }
                    }
                } else if n == 2 {
                    self.handle_mixed_pipeline(
                        &segments,
                        &stdout_redirect,
                        &stderr_redirect,
                    );
                }

                return true;
            }
        }

        // VARIABLE EXPANSION
        for part in parts.iter_mut() {
            *part = expand_variables(part, &self.variables);
        }
        parts.retain(|p| !p.is_empty());

        if parts.is_empty() {
            return true;
        }

        let command = parts[0].clone();
        let args = &parts[1..];

        match command.as_str() {
            "exit" => {
                return false;
            }
            "jobs" => {
                self.handle_jobs();
            }
            "history" => {
                self.handle_history(args);
            }
            "echo" => {
                self.handle_echo(args, &stdout_redirect, &stderr_redirect);
            }
            "pwd" => {
                println!("{}", env::current_dir().unwrap().display());
            }
            "cd" => {
                self.handle_cd(args);
            }
            "type" => {
                self.handle_type(args);
            }
            "declare" => {
                self.handle_declare(args);
            }
            "complete" => {
                self.handle_complete(args);
            }
            _ => {
                self.run_external(&command, args, &stdout_redirect, &stderr_redirect, background, input);
            }
        }

        true
    }

    fn handle_mixed_pipeline(
        &mut self,
        segments: &[&[String]],
        stdout_redirect: &Option<(String, bool)>,
        stderr_redirect: &Option<(String, bool)>,
    ) {
        let left_parts = &segments[0];
        let right_parts = &segments[1];
        let left_cmd = &left_parts[0];
        let left_args = &left_parts[1..];
        let right_cmd = &right_parts[0];
        let right_args = &right_parts[1..];

        let left_is_builtin = BUILTINS.contains(&left_cmd.as_str());
        let right_is_builtin = BUILTINS.contains(&right_cmd.as_str());

        if left_is_builtin {
            let output = match left_cmd.as_str() {
                "echo" => format!("{}\n", left_args.join(" ")),
                "pwd" => format!("{}\n", env::current_dir().unwrap().display()),
                "type" => {
                    let mut s = String::new();
                    if !left_args.is_empty() {
                        let cmd = &left_args[0];
                        if BUILTINS.contains(&cmd.as_str()) {
                            s = format!("{} is a shell builtin\n", cmd);
                        } else {
                            match find_executable(cmd) {
                                Some(path) => {
                                    s = format!("{} is {}\n", cmd, path.display());
                                }
                                None => {
                                    s = format!("{}: not found\n", cmd);
                                }
                            }
                        }
                    }
                    s
                }
                _ => String::new(),
            };

            if right_is_builtin {
                match right_cmd.as_str() {
                    "echo" => println!("{}", right_args.join(" ")),
                    "type" => {
                        if !right_args.is_empty() {
                            let cmd = &right_args[0];
                            if BUILTINS.contains(&cmd.as_str()) {
                                println!("{} is a shell builtin", cmd);
                            } else {
                                match find_executable(cmd) {
                                    Some(path) => println!("{} is {}", cmd, path.display()),
                                    None => println!("{}: not found", cmd),
                                }
                            }
                        }
                    }
                    "pwd" => println!("{}", env::current_dir().unwrap().display()),
                    "exit" => {
                        // Can't really exit here in this context, but preserve original behavior
                    }
                    _ => {}
                }
            } else {
                match find_executable(right_cmd) {
                    Some(right_path) => {
                        let mut right = Command::new(&right_path);
                        #[cfg(unix)]
                        {
                            right.arg0(right_cmd);
                        }
                        right.args(right_args);
                        right.stdin(Stdio::piped());

                        if let Some((file_path, append)) = stdout_redirect {
                            let file = open_file(file_path, *append);
                            right.stdout(Stdio::from(file));
                        }

                        if let Some((file_path, append)) = stderr_redirect {
                            let file = open_file(file_path, *append);
                            right.stderr(Stdio::from(file));
                        }

                        let mut right_child = match right.spawn() {
                            Ok(c) => c,
                            Err(_) => {
                                eprintln!("{}: command not found", right_cmd);
                                return;
                            }
                        };

                        if let Some(mut stdin) = right_child.stdin.take() {
                            let _ = stdin.write_all(output.as_bytes());
                        }

                        let _ = right_child.wait();
                    }
                    None => {
                        eprintln!("{}: command not found", right_cmd);
                    }
                }
            }
        } else if right_is_builtin {
            match find_executable(left_cmd) {
                Some(left_path) => {
                    let mut left = Command::new(&left_path);
                    #[cfg(unix)]
                    {
                        left.arg0(left_cmd);
                    }
                    left.args(left_args);
                    left.stdout(Stdio::null());

                    if let Some((file_path, append)) = stderr_redirect {
                        let file = open_file(file_path, *append);
                        left.stderr(Stdio::from(file));
                    }

                    let mut left_child = match left.spawn() {
                        Ok(c) => c,
                        Err(_) => {
                            eprintln!("{}: command not found", left_cmd);
                            return;
                        }
                    };

                    match right_cmd.as_str() {
                        "echo" => println!("{}", right_args.join(" ")),
                        "type" => {
                            if !right_args.is_empty() {
                                let cmd = &right_args[0];
                                if BUILTINS.contains(&cmd.as_str()) {
                                    println!("{} is a shell builtin", cmd);
                                } else {
                                    match find_executable(cmd) {
                                        Some(path) => println!("{} is {}", cmd, path.display()),
                                        None => println!("{}: not found", cmd),
                                    }
                                }
                            }
                        }
                        "pwd" => println!("{}", env::current_dir().unwrap().display()),
                        "exit" => {}
                        _ => {}
                    }

                    let _ = left_child.wait();
                }
                None => {
                    eprintln!("{}: command not found", left_cmd);
                }
            }
        } else {
            match find_executable(left_cmd) {
                Some(left_path) => match find_executable(right_cmd) {
                    Some(right_path) => {
                        let mut left = Command::new(&left_path);
                        #[cfg(unix)]
                        {
                            left.arg0(left_cmd);
                        }
                        left.args(left_args);
                        left.stdout(Stdio::piped());

                        if let Some((file_path, append)) = stderr_redirect {
                            let file = open_file(file_path, *append);
                            left.stderr(Stdio::from(file));
                        }

                        let mut left_child = match left.spawn() {
                            Ok(c) => c,
                            Err(_) => {
                                eprintln!("{}: command not found", left_cmd);
                                return;
                            }
                        };

                        let left_stdout = left_child.stdout.take().unwrap();

                        let mut right = Command::new(&right_path);
                        #[cfg(unix)]
                        {
                            right.arg0(right_cmd);
                        }
                        right.args(right_args);
                        right.stdin(Stdio::from(left_stdout));

                        if let Some((file_path, append)) = stdout_redirect {
                            let file = open_file(file_path, *append);
                            right.stdout(Stdio::from(file));
                        }

                        if let Some((file_path, append)) = stderr_redirect {
                            let file = open_file(file_path, *append);
                            right.stderr(Stdio::from(file));
                        }

                        let mut right_child = match right.spawn() {
                            Ok(c) => c,
                            Err(_) => {
                                eprintln!("{}: command not found", right_cmd);
                                let _ = left_child.kill();
                                return;
                            }
                        };

                        let _ = right_child.wait();
                        let _ = left_child.wait();
                    }
                    None => {
                        eprintln!("{}: command not found", right_cmd);
                    }
                },
                None => {
                    eprintln!("{}: command not found", left_cmd);
                }
            }
        }
    }

    fn handle_jobs(&mut self) {
        let mut running: Vec<bool> = Vec::with_capacity(self.jobs.len());
        for job in self.jobs.iter_mut() {
            running.push(job.child.try_wait().ok() == Some(None));
        }

        let max1 = self.jobs.iter().map(|j| j.id).max().unwrap_or(0);
        let max2 = self.jobs.iter()
            .filter(|j| j.id != max1)
            .map(|j| j.id)
            .max()
            .unwrap_or(0);

        for (i, &is_running) in running.iter().enumerate() {
            let marker = if self.jobs[i].id == max1 {
                '+'
            } else if self.jobs[i].id == max2 {
                '-'
            } else {
                ' '
            };

            if is_running {
                println!(
                    "[{}]{}  {:<24}{}",
                    self.jobs[i].id, marker, "Running", self.jobs[i].command
                );
            } else {
                let cmd = self.jobs[i]
                    .command
                    .trim_end()
                    .trim_end_matches('&')
                    .trim_end()
                    .to_string();
                println!(
                    "[{}]{}  {:<24}{}",
                    self.jobs[i].id, marker, "Done", cmd
                );
            }
        }

        let mut i = self.jobs.len();
        while i > 0 {
            i -= 1;
            if !running[i] {
                self.jobs.remove(i);
            }
        }
    }

    fn handle_history(&mut self, args: &[String]) {
        if args.len() >= 2 && args[0] == "-r" {
            let path = &args[1];
            if let Ok(contents) = fs::read_to_string(path) {
                for line in contents.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        self.cmd_history.push(trimmed.to_string());
                        let _ = self.rl.add_history_entry(trimmed);
                    }
                }
            }
        } else if args.len() >= 2 && args[0] == "-w" {
            let path = &args[1];
            let mut content = String::new();
            for entry in &self.cmd_history {
                content.push_str(entry);
                content.push('\n');
            }
            let _ = fs::write(path, content);
        } else if args.len() >= 2 && args[0] == "-a" {
            let path = &args[1];
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                for entry in self.cmd_history.iter().skip(self.last_written_count) {
                    let _ = writeln!(file, "{}", entry);
                }
                self.last_written_count = self.cmd_history.len();
            }
        } else {
            let n = args.first().and_then(|s| s.parse::<usize>().ok());
            let start = n.map(|n| self.cmd_history.len().saturating_sub(n)).unwrap_or(0);
            for (i, entry) in self.cmd_history.iter().enumerate().skip(start) {
                println!("{:>5}  {}", i + 1, entry);
            }
        }
    }

    fn handle_echo(
        &self,
        args: &[String],
        stdout_redirect: &Option<(String, bool)>,
        stderr_redirect: &Option<(String, bool)>,
    ) {
        let output = format!("{}\n", args.join(" "));

        if let Some((stderr_path, append)) = stderr_redirect {
            let _ = open_file(stderr_path, *append);
        }

        if let Some((stdout_path, append)) = stdout_redirect {
            let mut file = open_file(stdout_path, *append);
            file.write_all(output.as_bytes()).unwrap();
        } else {
            print!("{}", output);
        }
    }

    fn handle_cd(&self, args: &[String]) {
        if args.is_empty() {
            return;
        }

        let target = if args[0] == "~" {
            env::var("HOME").unwrap_or_default()
        } else {
            args[0].clone()
        };

        if env::set_current_dir(&target).is_err() {
            eprintln!("cd: {}: No such file or directory", target);
        }
    }

    fn handle_type(&self, args: &[String]) {
        if args.is_empty() {
            return;
        }

        let cmd = &args[0];
        if BUILTINS.contains(&cmd.as_str()) {
            println!("{} is a shell builtin", cmd);
        } else {
            match find_executable(cmd) {
                Some(path) => println!("{} is {}", cmd, path.display()),
                None => println!("{}: not found", cmd),
            }
        }
    }

    fn handle_declare(&mut self, args: &[String]) {
        if args.len() >= 2 && args[0] == "-p" {
            let name = &args[1];
            if let Some(value) = self.variables.get(name) {
                println!("declare -- {}=\"{}\"", name, value);
            } else {
                eprintln!("declare: {}: not found", name);
            }
        } else if let Some(eq_pos) = args.first().and_then(|a| a.find('=')) {
            let name = args[0][..eq_pos].to_string();
            let value = args[0][eq_pos + 1..].to_string();
            if !is_valid_identifier(&name) {
                eprintln!("declare: `{}': not a valid identifier", args[0]);
            } else {
                self.variables.insert(name, value);
            }
        }
    }

    fn handle_complete(&mut self, args: &[String]) {
        if args.len() >= 3 && args[0] == "-C" {
            let script = args[1].clone();
            let cmd = args[2].clone();
            if let Some(helper) = self.rl.helper_mut() {
                helper.completions.borrow_mut().insert(cmd, script);
            }
        } else if args.len() >= 2 && args[0] == "-r" {
            let cmd = &args[1];
            if let Some(helper) = self.rl.helper_mut() {
                helper.completions.borrow_mut().remove(cmd);
            }
        } else if args.len() >= 2 && args[0] == "-p" {
            let cmd = &args[1];
            if let Some(helper) = self.rl.helper_mut() {
                let completions = helper.completions.borrow();
                match completions.get(cmd) {
                    Some(path) => println!("complete -C '{}' {}", path, cmd),
                    None => println!("complete: {}: no completion specification", cmd),
                }
            }
        }
    }

    fn run_external(
        &mut self,
        command: &str,
        args: &[String],
        stdout_redirect: &Option<(String, bool)>,
        stderr_redirect: &Option<(String, bool)>,
        background: bool,
        input: &str,
    ) {
        match find_executable(command) {
            Some(path) => {
                let mut cmd = Command::new(&path);
                #[cfg(unix)]
                {
                    cmd.arg0(command);
                }
                cmd.args(args);

                if let Some((file_path, append)) = stdout_redirect {
                    let file = open_file(file_path, *append);
                    cmd.stdout(Stdio::from(file));
                }

                if let Some((file_path, append)) = stderr_redirect {
                    let file = open_file(file_path, *append);
                    cmd.stderr(Stdio::from(file));
                }

                match cmd.spawn() {
                    Ok(mut child) => {
                        if background {
                            let job_id = (1..)
                                .find(|id| !self.jobs.iter().any(|j| j.id == *id))
                                .unwrap();
                            let pid = child.id();
                            println!("[{}] {}", job_id, pid);
                            self.jobs.push(Job {
                                id: job_id,
                                command: input.to_string(),
                                child,
                            });
                        } else {
                            child.wait().unwrap();
                        }
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
