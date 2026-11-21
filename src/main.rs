mod event;
mod process;

use std::sync::mpsc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();

    let (reaper_handle, reaper_thread) =
        process::start_child_reaper(tx.clone())?;

    reaper_handle.close();
    reaper_thread.join().expect("Reaper thread panicked");

    Ok(())
}