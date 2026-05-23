// src/main.rs

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};

// Shell completer
struct ShellCompleter;

// Implement Helper manually
impl Helper for ShellCompleter {}

// Optional trait impls
impl Hinter for ShellCompleter {
    type Hint = String;
}

impl Highlighter for ShellCompleter {}

impl Validator for ShellCompleter {}

// TAB completion
impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let builtins = ["echo", "exit"];

        let input = &line[..pos];

        let mut matches = Vec::new();

        for builtin in builtins {
            if builtin.starts_with(input) {
                matches.push(Pair {
                    display: builtin.to_string(),

                    // Add trailing space
                    replacement: format!("{} ", builtin),
                });
            }
        }

        Ok((0, matches))
    }
}

fn main() {
    // Editor requires DefaultHistory in rustyline v14
    let mut rl = Editor::<ShellCompleter, DefaultHistory>::new().unwrap();

    rl.set_helper(Some(ShellCompleter));

    loop {
        let readline = rl.readline("$ ");

        match readline {
            Ok(line) => {
                let input = line.trim();

                // exit builtin
                if input == "exit" {
                    break;
                }

                // echo builtin
                if input.starts_with("echo ") {
                    println!("{}", &input[5..]);
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
