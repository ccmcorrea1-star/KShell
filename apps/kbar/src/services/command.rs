//! Small, bounded subprocess helpers used by system service backends.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const COMMAND_TIMEOUT: Duration = Duration::from_millis(500);
const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(5);
const COMMAND_OUTPUT_DRAIN_TIMEOUT: Duration = Duration::from_millis(50);

pub fn run_controlled(program: &str, args: &[String]) -> bool {
    let Some(child) = spawn(program, args, Stdio::null()) else {
        return false;
    };
    wait_for_command(child)
}

pub fn output(program: &str, args: &[&str]) -> Option<String> {
    let owned_args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    let mut child = spawn(program, &owned_args, Stdio::piped())?;
    let Some(mut stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    };

    let (output_sender, output_receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut output = String::new();
        let result = stdout.read_to_string(&mut output).map(|_| output);
        let _ = output_sender.send(result);
    });

    let success = wait_for_command(child);
    let output = output_receiver
        .recv_timeout(COMMAND_OUTPUT_DRAIN_TIMEOUT)
        .ok()?
        .ok()?;
    success.then_some(output)
}

fn spawn(program: &str, args: &[String], stdout: Stdio) -> Option<Child> {
    Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdout(stdout)
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

fn wait_for_command(mut child: Child) -> bool {
    let deadline = Instant::now() + COMMAND_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(COMMAND_POLL_INTERVAL),
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::run_controlled;

    #[test]
    fn controlled_commands_do_not_wait_forever() {
        let started = Instant::now();
        assert!(!run_controlled("sleep", &["2".to_owned()]));
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
