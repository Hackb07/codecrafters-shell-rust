// src/main.rs

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};

#[derive(Helper, Hinter, Validator, Highlighter)]
struct ShellCompleter;

// Custom TAB completion
impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // Builtin commands
        let builtins = ["echo", "exit"];

        // Current typed text
        let input = &line[..pos];

        let mut matches = Vec::new();

        for builtin in builtins {
            // Match partial input
            if builtin.starts_with(input) {
                matches.push(Pair {
                    display: builtin.to_string(),

                    // Add trailing space
                    replacement: format!("{} ", builtin),
                });
            }
        }

        // Replace from beginning of line
        Ok((0, matches))
    }
}

fn main() {
    // Create rustyline editor
    let mut rl = Editor::<ShellCompleter>::new().unwrap();

    // Attach completer
    rl.set_helper(Some(ShellCompleter));

    loop {
        // Read line with prompt
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
