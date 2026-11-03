use std::io::{self, Write};
use std::process::{Child, Command, Stdio};

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

        let mut children: Vec<Child> = Vec::new();
        let mut previous_stdout = None;
        let mut last_pipe_start = 0;
        let mut failed = false;

        for i in 0..=tokens.len() {
            if i != tokens.len() && tokens[i] != "|" {
                continue;
            }

            if last_pipe_start == i {
                eprintln!("syntax error: empty command in pipeline");
                failed = true;
                break;
            }

            let program = tokens[last_pipe_start];
            let args = &tokens[last_pipe_start + 1..i];
            let has_next = i < tokens.len();

            let mut cmd = Command::new(program);
            cmd.args(args);

            if let Some(stdout) = previous_stdout.take() {
                cmd.stdin(Stdio::from(stdout));
            }

            if has_next {
                cmd.stdout(Stdio::piped());
            }

            match cmd.spawn() {
                Ok(mut child) => {
                    previous_stdout = child.stdout.take();
                    children.push(child);
                }
                Err(e) => {
                    eprintln!("{program}: {e}");
                    failed = true;
                    break;
                }
            }

            last_pipe_start = i + 1;
        }

        if failed {
            drop(previous_stdout);
        }

        for mut child in children {
            if let Err(e) = child.wait() {
                eprintln!("wait: {e}");
            }
        }
    }
}