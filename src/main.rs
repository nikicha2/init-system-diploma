mod event;
mod process;
mod event_loop;
mod config;

use std::{sync::mpsc, thread::sleep, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let services = config::ordered_processes();

    let (tx, rx) = mpsc::channel();

    let (reaper_handle, reaper_thread) =
        process::start_child_reaper(tx.clone())?;

    let states = process::start_services(&services);

    println!("Service states: {:?}", states);
    
    drop(tx);

    event_loop::run(rx);

    reaper_handle.close();
    reaper_thread.join().expect("Reaper thread panicked");

    Ok(())
}