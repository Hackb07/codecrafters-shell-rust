#[allow(unused_imports)]
use std::io::{self, BufRead, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        command = command.trim().to_string();
        if command == "exit" {
            break;
        }
        let parts: Vec<&str> = command.split_whitespace().collect();
        if !parts.is_empty() && parts[0] == "echo" {
            let args = parts[1..].join(" ");
            println!("{}", args);
        } else {
            println!("{}: command not found", command);
        }
    }
}
