use std::{
    process::{Command, Stdio},
    thread,
    time::Duration,
};

fn main() {
    println!(" Rust init system started");
    println!(" PID: {}", std::process::id());

    println!("Starting BusyBox shell...");

    let status = Command::new("/bin/sh")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    println!("Shell exited: {status:?}");

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}