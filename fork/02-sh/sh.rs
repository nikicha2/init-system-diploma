use std::io::{self, Write};
use std::process::Command;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();

        match io::stdin().read_line(&mut command) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("read_line: {e}");
                continue;
            }
        }

        let tokens: Vec<&str> = command.split_whitespace().collect();

        if tokens.is_empty() {
            continue;
        }

        if let Err(e) = Command::new(tokens[0])
            .args(&tokens[1..])
            .status()
        {
            eprintln!("{}: {}", tokens[0], e);
        }
    }
}