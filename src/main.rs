use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};

#[derive(Helper, Completer, Hinter, Validator, Highlighter)]
struct ShellHelper;

impl rustyline::completion::Completer for ShellHelper {
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

        for cmd in builtins {
            if cmd.starts_with(input) {
                matches.push(Pair {
                    display: cmd.to_string(),
                    replacement: format!("{} ", cmd),
                });
            }
        }

        Ok((0, matches))
    }
}

fn main() -> rustyline::Result<()> {
    let helper = ShellHelper;

    let mut rl = Editor::new()?;

    rl.set_helper(Some(helper));

    loop {
        let line = rl.readline("$ ");

        match line {
            Ok(input) => {
                let input = input.trim();

                if input == "exit" {
                    break;
                }

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
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}
