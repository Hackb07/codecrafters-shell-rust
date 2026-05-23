use std::io::{self, Write};

fn main() {
    loop {
        // Print shell prompt
        print!(">> ");
        io::stdout().flush().unwrap();

        // Read user input
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();

        let command = command.trim();

        // Skip empty input
        if command.is_empty() {
            continue;
        }

        // Split command into parts
        let parts: Vec<&str> = command.split_whitespace().collect();

        match parts[0] {
            "exit" => {
                break;
            }

            "echo" => {
                let args = parts[1..].join(" ");
                println!("{}", args);
            }

            "type" => {
                if parts.len() < 2 {
                    println!("type: missing argument");
                    continue;
                }

                match parts[1] {
                    "echo" => println!("echo is a shell builtin"),
                    "exit" => println!("exit is a shell builtin"),
                    "type" => println!("type is a shell builtin"),
                    _ => println!("{}: not found", parts[1]),
                }
            }

            _ => {
                println!("{}: command not found", command);
            }
        }
    }
}
