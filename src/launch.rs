use std::io;
use std::process::{Child, Command};

use crate::desktop::DesktopEntry;

pub fn launch(application: &DesktopEntry) -> io::Result<Child> {
    let (program, arguments) = application
        .exec
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty Exec field"))?;

    let mut command = Command::new(program);
    command.args(arguments);
    if let Some(working_dir) = &application.working_dir {
        command.current_dir(working_dir);
    }
    command.spawn()
}
