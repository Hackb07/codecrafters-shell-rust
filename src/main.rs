use std::env::args;
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
        if !parts.is_empty() && parts[0] == "type" {
            if !parts.is_empty() && parts[0] == "echo" {
                let args = parts[1..].join(" ");
                println!("{}", args);
            } else if !parts.is_empty() && parts[0] == "type" {
                println!("type is a shell builtin");
            } else if !parts.is_empty() && parts[0] == "exit" {
                println!("exit is a shell builtin");
            } else {
                println!("{:?}: not found", args);
            }
        }
        {
            println!("{}: command not found", command);
        }
    }
}
