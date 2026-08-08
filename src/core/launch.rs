use std::ffi::{OsStr, OsString};
use std::io;
use std::process::{Child, Command, Stdio};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use super::desktop::DesktopEntry;

#[derive(Debug, PartialEq, Eq)]
struct CommandSpec {
    program: OsString,
    arguments: Vec<OsString>,
}

pub fn launch(application: &DesktopEntry) -> io::Result<Child> {
    let terminal = application
        .terminal
        .then(|| std::env::var_os("TERMINAL"))
        .flatten();
    let spec = build_command(application, terminal.as_deref())?;

    let mut command = Command::new(&spec.program);
    command.args(&spec.arguments);
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

fn build_command(application: &DesktopEntry, terminal: Option<&OsStr>) -> io::Result<CommandSpec> {
    let (program, arguments) = application
        .exec
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty Exec field"))?;
    let program = OsString::from(program);
    let arguments = arguments.iter().map(OsString::from).collect::<Vec<_>>();

    if application.terminal {
        let terminal = terminal
            .filter(|terminal| !terminal.is_empty())
            .unwrap_or_else(|| OsStr::new("kitty"));
        let mut terminal_arguments = Vec::with_capacity(arguments.len() + 1);
        terminal_arguments.push(program);
        terminal_arguments.extend(arguments);
        return Ok(CommandSpec {
            program: terminal.to_owned(),
            arguments: terminal_arguments,
        });
    }

    Ok(CommandSpec { program, arguments })
}

#[cfg(test)]
mod tests {
    use super::super::desktop::DesktopEntry;
    use super::{build_command, launch};
    use std::ffi::{OsStr, OsString};

    fn application(terminal: bool) -> DesktopEntry {
        DesktopEntry {
            name: "Test application".to_owned(),
            generic_name: None,
            icon: None,
            exec: vec![
                "app".to_owned(),
                "--title".to_owned(),
                "A title with spaces".to_owned(),
            ],
            working_dir: None,
            terminal,
        }
    }

    #[test]
    fn terminal_command_uses_configured_terminal_and_preserves_arguments() {
        let spec = build_command(&application(true), Some(OsStr::new("foot")))
            .expect("terminal command specification");

        assert_eq!(spec.program, OsStr::new("foot"));
        assert_eq!(
            spec.arguments,
            vec![
                OsString::from("app"),
                OsString::from("--title"),
                OsString::from("A title with spaces"),
            ]
        );
    }

    #[test]
    fn terminal_command_falls_back_to_kitty() {
        let spec = build_command(&application(true), None).expect("terminal command specification");

        assert_eq!(spec.program, OsStr::new("kitty"));
        assert_eq!(spec.arguments[0], OsString::from("app"));
    }

    #[test]
    fn non_terminal_command_keeps_exec_program_and_arguments() {
        let spec = build_command(&application(false), Some(OsStr::new("must-not-be-used")))
            .expect("direct command specification");

        assert_eq!(spec.program, OsStr::new("app"));
        assert_eq!(
            spec.arguments,
            vec![
                OsString::from("--title"),
                OsString::from("A title with spaces"),
            ]
        );
    }

    #[test]
    fn rejects_an_empty_exec_field_before_spawning() {
        let application = DesktopEntry {
            name: "Invalid".to_owned(),
            generic_name: None,
            icon: None,
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
    fn puts_non_terminal_processes_in_a_new_session() {
        let executable = std::env::current_exe().expect("test executable path");
        let parent_session = unsafe { libc::getsid(0) };
        assert_ne!(parent_session, -1, "parent session must be available");

        let application = DesktopEntry {
            name: "Session test".to_owned(),
            generic_name: None,
            icon: None,
            exec: vec![
                executable.to_string_lossy().into_owned(),
                "--exact".to_owned(),
                "launch::tests::holds_for_session_configuration_test".to_owned(),
                "--ignored".to_owned(),
            ],
            working_dir: None,
            terminal: false,
        };
        let mut child = launch(&application).expect("session test child");
        let child_session = unsafe { libc::getsid(child.id() as libc::pid_t) };
        let _ = child.kill();
        let _ = child.wait();

        assert_ne!(child_session, -1, "child session must be available");
        assert_ne!(child_session, parent_session);
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore]
    fn holds_for_session_configuration_test() {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
