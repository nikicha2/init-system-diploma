use std::{
    io,
    sync::mpsc::Sender,
    thread::{self, JoinHandle},
};

use signal_hook::{
    consts::SIGCHLD,
    iterator::{Handle as SignalsHandle, Signals},
};

use crate::event::Event;

pub(crate) fn start_child_reaper(tx: Sender<Event>) -> io::Result<(SignalsHandle, JoinHandle<()>)> {
    // Register the SIGCHLD listener.
    let mut signals = Signals::new([SIGCHLD])?;

    let handle = signals.handle();

    let thread = thread::spawn(move || {
        for _signal in signals.forever() {
            loop {
                let mut status: libc::c_int = 0;

                let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };

                if pid > 0 {
                    if tx.send(Event::ChildExited { pid, status }).is_err() {
                        return;
                    }

                    continue;
                }

                if pid == 0 {
                    break;
                }

                let error = io::Error::last_os_error();

                if error.raw_os_error() == Some(libc::EINTR) {
                    // Interrupted system call. Try again.
                    continue;
                }

                if error.raw_os_error() == Some(libc::ECHILD) {
                    // There are currently no children to reap.
                    break;
                }

                // Unexpected error.
                let _ = tx.send(Event::ReaperError(error.to_string()));
                return;
            }
        }
    });

    Ok((handle, thread))
}
