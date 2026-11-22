use std::sync::mpsc::Receiver;

use crate::event::Event;

pub(crate) fn run(rx: Receiver<Event>) {
    loop {
        match rx.recv() {
            Ok(event) => match event {
                Event::ChildExited { pid, status } => {
                    if libc::WIFEXITED(status) {
                        let exit_code = libc::WEXITSTATUS(status);
                        println!("Child {pid} exited with code {exit_code}");
                    } else if libc::WIFSIGNALED(status) {
                        let signal = libc::WTERMSIG(status);
                        println!("Child {pid} terminated by signal {signal}");
                    }
                }

                Event::ReaperError(error) => {
                    eprintln!("Reaper error: {error}");
                }
            },

            Err(_) => {
                eprintln!("Event channel disconnected");
                break;
            }
        }
    }
}
