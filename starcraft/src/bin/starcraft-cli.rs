
use std::env;
use std::io::{self, BufReader, Read, Write};
use std::os::unix::net::UnixStream;

const SOCKET_PATH: &str = "/tmp/starcraft.sock";

fn main() -> io::Result<()> {
    let command = env::args().nth(1);

    let Some(command) = command else {
        eprintln!("Usage: starcraft-cli <status|exit>");
        return Ok(());
    };

    if command != "status" && command != "exit" {
        eprintln!("Unknown command. Available commands: status, exit");
        return Ok(());
    }

    let mut stream = UnixStream::connect(SOCKET_PATH)?;

    writeln!(stream, "{command}")?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_to_string(&mut response)?;

    print!("{response}");

    Ok(())
}
