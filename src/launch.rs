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
    if !application.terminal {
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    command.spawn()
}

#[cfg(test)]
mod tests {
    use super::launch;
    use crate::desktop::DesktopEntry;

    #[test]
    fn rejects_an_empty_exec_field_before_spawning() {
        let application = DesktopEntry {
            name: "Invalid".to_owned(),
            generic_name: None,
            exec: Vec::new(),
            working_dir: None,
            terminal: false,
        };

        let error = match launch(&application) {
            Ok(mut child) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("empty Exec must not spawn a process");
            }
            Err(error) => error,
        };
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn keeps_terminal_processes_in_the_parent_session() {
        let executable = std::env::current_exe().expect("test executable path");
        let parent_session = unsafe { libc::getsid(0) };
        assert_ne!(parent_session, -1, "parent session must be available");

        for terminal in [false, true] {
            let application = DesktopEntry {
                name: "Session test".to_owned(),
                generic_name: None,
                exec: vec![
                    executable.to_string_lossy().into_owned(),
                    "--exact".to_owned(),
                    "launch::tests::holds_for_session_configuration_test".to_owned(),
                    "--ignored".to_owned(),
                ],
                working_dir: None,
                terminal,
            };
            let mut child = launch(&application).expect("session test child");
            let child_session = unsafe { libc::getsid(child.id() as libc::pid_t) };
            let _ = child.kill();
            let _ = child.wait();

            assert_ne!(child_session, -1, "child session must be available");
            assert_eq!(child_session == parent_session, terminal);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore]
    fn holds_for_session_configuration_test() {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
