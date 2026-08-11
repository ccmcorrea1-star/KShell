//! Blocking Unix-socket connection with a persistent, reconnecting event stream.

use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crate::protocol::{
    event_stream_request, focus_workspace_reference_request, WorkspaceReference,
};
use crate::state::WorkspaceState;

const INITIAL_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(2);
const STABLE_CONNECTION: Duration = Duration::from_secs(1);

pub fn spawn_event_stream<F>(publish: F) -> thread::JoinHandle<()>
where
    F: Fn(WorkspaceState) -> bool + Send + 'static,
{
    thread::spawn(move || run_event_stream(publish))
}

pub fn focus_workspace(index: usize) {
    if index == 0 {
        return;
    }
    focus_workspace_reference(WorkspaceReference::Index(index));
}

pub fn focus_workspace_id(id: u64) {
    focus_workspace_reference(WorkspaceReference::Id(id));
}

fn focus_workspace_reference(reference: WorkspaceReference) {
    let Some(socket_path) = socket_path() else {
        return;
    };

    thread::spawn(move || {
        if let Err(error) = send_focus_request(&socket_path, reference) {
            eprintln!("Niri focus request failed: {error}");
        }
    });
}

fn run_event_stream<F>(publish: F)
where
    F: Fn(WorkspaceState) -> bool,
{
    let mut backoff = INITIAL_BACKOFF;
    let mut had_state = false;

    loop {
        let Some(socket_path) = socket_path() else {
            thread::sleep(backoff);
            backoff = next_backoff(backoff);
            continue;
        };

        let connected_at = Instant::now();
        let result = stream_once(&socket_path, &publish);
        if result.receiver_closed {
            return;
        }
        if result.published {
            had_state = true;
        }
        if !reset_after_disconnect(&mut had_state, &publish) {
            return;
        }

        backoff = if connected_at.elapsed() >= STABLE_CONNECTION {
            INITIAL_BACKOFF
        } else {
            next_backoff(backoff)
        };
        thread::sleep(backoff);
    }
}

fn reset_after_disconnect<F>(had_state: &mut bool, publish: &F) -> bool
where
    F: Fn(WorkspaceState) -> bool,
{
    if !*had_state {
        return true;
    }
    *had_state = false;
    publish(WorkspaceState::default())
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct StreamResult {
    published: bool,
    receiver_closed: bool,
}

fn stream_once<F>(socket_path: &Path, publish: &F) -> StreamResult
where
    F: Fn(WorkspaceState) -> bool,
{
    let Ok(mut stream) = UnixStream::connect(socket_path) else {
        return StreamResult::default();
    };
    let Ok(request) = event_stream_request() else {
        return StreamResult::default();
    };
    if stream.write_all(&request).is_err() {
        return StreamResult::default();
    }

    let reader = BufReader::new(stream);
    let mut state = WorkspaceState::default();
    let mut has_snapshot = false;
    let mut published = false;
    for line in reader.lines() {
        let Ok(line) = line else {
            break;
        };
        let Ok(event) = crate::protocol::parse_event(&line) else {
            continue;
        };
        let Some(next) = state.apply_event(event) else {
            continue;
        };
        let changed = !has_snapshot || next != state;
        state = next;
        has_snapshot = true;
        if !changed {
            continue;
        }
        published = true;
        if !publish(state.clone()) {
            return StreamResult {
                published,
                receiver_closed: true,
            };
        }
    }

    StreamResult {
        published,
        receiver_closed: false,
    }
}

fn socket_path() -> Option<PathBuf> {
    env::var_os("NIRI_SOCKET").map(PathBuf::from)
}

fn send_focus_request(socket_path: &Path, reference: WorkspaceReference) -> io::Result<()> {
    let mut stream = UnixStream::connect(socket_path)?;
    let request = focus_workspace_reference_request(reference)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    stream.write_all(&request)
}

fn next_backoff(current: Duration) -> Duration {
    let next = current.saturating_mul(2);
    next.min(MAX_BACKOFF)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::time::Duration;

    use super::{next_backoff, reset_after_disconnect, INITIAL_BACKOFF, MAX_BACKOFF};
    use crate::WorkspaceState;

    #[test]
    fn reconnect_backoff_is_limited_and_starts_at_250ms() {
        assert_eq!(INITIAL_BACKOFF, Duration::from_millis(250));
        assert_eq!(next_backoff(INITIAL_BACKOFF), Duration::from_millis(500));
        assert_eq!(
            next_backoff(Duration::from_millis(500)),
            Duration::from_secs(1)
        );
        assert_eq!(next_backoff(Duration::from_secs(1)), MAX_BACKOFF);
        assert_eq!(next_backoff(MAX_BACKOFF), MAX_BACKOFF);
    }

    #[test]
    fn disconnect_reset_removes_stale_state_before_resync() {
        let mut had_state = true;
        let published = RefCell::new(Vec::new());
        assert!(reset_after_disconnect(&mut had_state, &|state| {
            published.borrow_mut().push(state);
            true
        }));
        assert!(!had_state);
        assert_eq!(*published.borrow(), vec![WorkspaceState::default()]);

        assert!(reset_after_disconnect(&mut had_state, &|state| {
            published.borrow_mut().push(state);
            true
        }));
        assert_eq!(published.borrow().len(), 1);
    }
}
