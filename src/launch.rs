use std::io;
use std::process::{Child, Command, Stdio};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

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
    if application.terminal {
        command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
    } else {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
    }

    #[cfg(unix)]
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    command.spawn()
}
