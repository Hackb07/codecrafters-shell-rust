// src/main.rs

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{
    Cmd, CompletionType, Config, Context, Editor, Helper, KeyCode, KeyEvent, Modifiers,
};

struct ShellCompleter;

// Required trait impls
impl Helper for ShellCompleter {}

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

        let matches: Vec<Pair> = builtins
            .iter()
            .filter(|cmd| cmd.starts_with(input))
            .map(|cmd| Pair {
                display: cmd.to_string(),

                // Replace fully with trailing space
                replacement: format!("{} ", cmd),
            })
            .collect();

        Ok((0, matches))
    }
}

fn main() {
    // Configure rustyline
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();

    let mut rl = Editor::<ShellCompleter, DefaultHistory>::with_config(config).unwrap();

    rl.set_helper(Some(ShellCompleter));

    // Force TAB completion
    rl.bind_sequence(KeyEvent(KeyCode::Tab, Modifiers::NONE), Cmd::Complete);

    loop {
        let readline = rl.readline("$ ");

        match readline {
            Ok(line) => {
                let input = line.trim();

                // exit
                if input == "exit" {
                    break;
                }

                // echo
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
