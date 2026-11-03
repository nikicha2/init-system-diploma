
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender, RecvTimeoutError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub const SOCKET_PATH: &str = "/tmp/starcraft.sock";

#[derive(Debug)]
pub enum GameCommand {
    Status,
    Exit,
}

pub struct SocketRequest {
    pub command: GameCommand,
    pub response: SyncSender<String>,
}

pub struct SocketServer {
    shutdown: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl SocketServer {
    pub fn stop(self) -> io::Result<()> {
        self.shutdown.store(true, Ordering::Relaxed);

        self.thread
            .join()
            .map_err(|_| io::Error::other("Socket server thread panicked"))?;

        match std::fs::remove_file(SOCKET_PATH) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

pub fn start_server(tx: Sender<crate::Event>) -> io::Result<SocketServer> {
    let listener = UnixListener::bind(SOCKET_PATH)?;
    listener.set_nonblocking(true)?;

    let shutdown = Arc::new(AtomicBool::new(false));
    let thread_shutdown = Arc::clone(&shutdown);

    let thread = thread::spawn(move || {
        while !thread_shutdown.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if let Err(error) = handle_client(stream, &tx, &thread_shutdown) {
                        eprintln!("Socket client error: {error}");
                    }
                }

                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }

                Err(error) => {
                    eprintln!("Socket server error: {error}");
                    break;
                }
            }
        }
    });

    Ok(SocketServer { shutdown, thread })
}

fn handle_client(
    mut stream: UnixStream,
    tx: &Sender<crate::Event>,
    shutdown: &AtomicBool,
) -> io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut input = String::new();

    if reader.read_line(&mut input)? == 0 {
        return Ok(());
    }

    let command = match input.trim() {
        "status" => GameCommand::Status,
        "exit" => GameCommand::Exit,
        _ => {
            writeln!(stream, "Unknown command.")?;
            return Ok(());
        }
    };

    let (response_tx, response_rx) = mpsc::sync_channel(1);

    tx.send(crate::Event::Socket(SocketRequest {
        command,
        response: response_tx,
    }))
    .map_err(|_| io::Error::other("Game event loop is unavailable"))?;

    loop {
        if shutdown.load(Ordering::Relaxed) {
            return Ok(());
        }

        match response_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(response) => {
                writeln!(stream, "{response}")?;
                return Ok(());
            }

            Err(RecvTimeoutError::Timeout) => continue,

            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::other("Game did not respond to command"));
            }
        }
    }
}
