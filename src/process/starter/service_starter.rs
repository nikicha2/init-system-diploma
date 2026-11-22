use std::io;
use std::process::Command;

use crate::config::ServiceConfig;

pub(super) fn start_service(service: &ServiceConfig) -> io::Result<u32> {
    let child = Command::new(&service.program)
        .args(&service.args)
        .spawn()?;

    Ok(child.id())
}