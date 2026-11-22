use std::sync::mpsc::Receiver;

use crate::event::Event;

pub(crate) fn run(rx: Receiver<Event>) {
    loop {
        match rx.recv() {
            Ok(event) => {
                match event {
                    Event::ChildExited { pid, status } => {
                        println!("Child {pid} exited with status {status}");
                    }

                    Event::ReaperError(error) => {
                        eprintln!("Reaper error: {error}");
                    }
                }
            }

            Err(_) => {
                eprintln!("Event channel disconnected");
                break;
            }
        }
    }
}