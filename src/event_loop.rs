use std::sync::mpsc::Receiver;

use crate::{event::Event, process::ServiceRegistry};

pub(crate) fn run(rx: Receiver<Event>, registry: &mut ServiceRegistry) {
    loop {
        match rx.recv() {
            Ok(event) => match event {
                Event::ChildExited { pid, status } => {
                    if let Some((service_name, process_exit)) =
                        registry.handle_child_exit(pid, status)
                    {
                        println!("Service {service_name} (PID {pid}) {process_exit}");
                    } else {
                        eprintln!("Received exit event for unknown child PID {pid}");
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
