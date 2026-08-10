use std::collections::HashMap;
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use serde_json::Value;

const EVENT_STREAM_REQUEST: &[u8] = b"\"EventStream\"\n";
pub const WORKSPACE_COUNT: usize = 5;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceState {
    pub focused_index: Option<usize>,
    workspaces: HashMap<u64, WorkspaceInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkspaceInfo {
    index: usize,
    output: Option<String>,
    is_active: bool,
    is_focused: bool,
}

pub fn spawn_event_stream(sender: Sender<WorkspaceState>) {
    thread::spawn(move || loop {
        let Some(socket_path) = socket_path() else {
            return;
        };

        let Ok(mut stream) = UnixStream::connect(&socket_path) else {
            thread::sleep(Duration::from_secs(2));
            continue;
        };

        if stream.write_all(EVENT_STREAM_REQUEST).is_err() {
            thread::sleep(Duration::from_secs(2));
            continue;
        }

        let reader = BufReader::new(stream);
        let mut state = WorkspaceState::default();
        for line in reader.lines() {
            let Ok(line) = line else {
                break;
            };
            let Some(next) = parse_workspace_event(&line, &state) else {
                continue;
            };
            state = next.clone();
            if sender.send(next).is_err() {
                return;
            }
        }

        thread::sleep(Duration::from_secs(2));
    });
}

pub fn focus_workspace(index: usize) {
    if !(1..=WORKSPACE_COUNT).contains(&index) {
        return;
    }

    let Some(socket_path) = socket_path() else {
        return;
    };
    thread::spawn(move || {
        let _ = send_focus_request(&socket_path, index);
    });
}

fn socket_path() -> Option<PathBuf> {
    env::var_os("NIRI_SOCKET").map(PathBuf::from)
}

fn send_focus_request(socket_path: &PathBuf, index: usize) -> io::Result<()> {
    let mut stream = UnixStream::connect(socket_path)?;
    let request = focus_request(index);
    stream.write_all(request.as_bytes())
}

fn focus_request(index: usize) -> String {
    format!("{{\"Action\":{{\"FocusWorkspace\":{{\"reference\":{{\"Index\":{index}}}}}}}}}\n")
}

fn parse_workspace_event(line: &str, current: &WorkspaceState) -> Option<WorkspaceState> {
    let message: Value = serde_json::from_str(line).ok()?;

    if let Some(payload) = message.get("WorkspacesChanged") {
        return parse_workspaces_changed(payload);
    }

    let payload = message.get("WorkspaceActivated")?;
    let id = payload.get("id").and_then(Value::as_u64)?;
    let focused = payload.get("focused").and_then(Value::as_bool)?;
    let workspace = current.workspaces.get(&id)?;
    let mut next = current.clone();
    let output = workspace.output.clone();

    for (workspace_id, workspace) in next.workspaces.iter_mut() {
        if workspace.output == output {
            workspace.is_active = *workspace_id == id;
        }
        if focused {
            workspace.is_focused = *workspace_id == id;
        }
    }
    next.refresh_focused_index();
    Some(next)
}

fn parse_workspaces_changed(payload: &Value) -> Option<WorkspaceState> {
    let workspaces = payload.get("workspaces")?.as_array()?;
    let mut state = WorkspaceState::default();

    for workspace in workspaces {
        let Some(id) = workspace.get("id").and_then(Value::as_u64) else {
            continue;
        };
        let Some(index) = workspace
            .get("idx")
            .and_then(Value::as_u64)
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| index.checked_sub(1))
        else {
            continue;
        };

        state.workspaces.insert(
            id,
            WorkspaceInfo {
                index,
                output: workspace
                    .get("output")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                is_active: workspace
                    .get("is_active")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                is_focused: workspace
                    .get("is_focused")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            },
        );
    }

    state.refresh_focused_index();
    Some(state)
}

impl WorkspaceState {
    fn refresh_focused_index(&mut self) {
        self.focused_index = self
            .workspaces
            .values()
            .find(|workspace| workspace.is_active && workspace.is_focused)
            .or_else(|| {
                self.workspaces
                    .values()
                    .find(|workspace| workspace.is_focused)
            })
            .map(|workspace| workspace.index);
    }
}

#[cfg(test)]
mod tests {
    use super::{focus_request, parse_workspace_event, WorkspaceState};

    #[test]
    fn encodes_the_niri_focus_workspace_request_without_a_shell() {
        assert_eq!(
            focus_request(3),
            "{\"Action\":{\"FocusWorkspace\":{\"reference\":{\"Index\":3}}}}\n"
        );
    }

    #[test]
    fn parses_the_initial_workspace_snapshot_and_focus() {
        let line = r#"{
            "WorkspacesChanged": {
                "workspaces": [
                    {"id": 11, "idx": 1, "output": "A", "is_active": false, "is_focused": false},
                    {"id": 12, "idx": 2, "output": "A", "is_active": true, "is_focused": true},
                    {"id": 13, "idx": 3, "output": "A", "is_active": false, "is_focused": false},
                    {"id": 14, "idx": 4, "output": "A", "is_active": false, "is_focused": false},
                    {"id": 15, "idx": 5, "output": "A", "is_active": false, "is_focused": false}
                ]
            }
        }"#;

        let state = parse_workspace_event(line, &WorkspaceState::default()).expect("snapshot");
        assert_eq!(state.focused_index, Some(1));
        assert!(state.workspaces.get(&12).expect("workspace").is_active);

        let activation = r#"{"WorkspaceActivated":{"id":13,"focused":true}}"#;
        let state = parse_workspace_event(activation, &state).expect("activation");
        assert_eq!(state.focused_index, Some(2));
        assert!(!state.workspaces.get(&12).expect("workspace").is_active);
        assert!(state.workspaces.get(&13).expect("workspace").is_active);
        assert!(!state.workspaces.get(&12).expect("workspace").is_focused);
    }

    #[test]
    fn applies_unfocused_activation_without_changing_the_focused_workspace() {
        let snapshot = r#"{
            "WorkspacesChanged": {
                "workspaces": [
                    {"id": 21, "idx": 1, "output": "A", "is_active": true, "is_focused": true},
                    {"id": 22, "idx": 2, "output": "A", "is_active": false, "is_focused": false},
                    {"id": 31, "idx": 1, "output": "B", "is_active": true, "is_focused": false}
                ]
            }
        }"#;
        let state = parse_workspace_event(snapshot, &WorkspaceState::default()).expect("snapshot");
        let activation = r#"{"WorkspaceActivated":{"id":22,"focused":false}}"#;
        let state = parse_workspace_event(activation, &state).expect("activation");

        assert_eq!(state.focused_index, Some(0));
        assert!(!state.workspaces.get(&21).expect("workspace").is_active);
        assert!(state.workspaces.get(&22).expect("workspace").is_active);
        assert!(state.workspaces.get(&31).expect("workspace").is_active);
    }

    #[test]
    fn replaces_the_workspace_snapshot_so_removed_active_state_cannot_linger() {
        let first = r#"{
            "WorkspacesChanged": {
                "workspaces": [
                    {"id": 41, "idx": 1, "output": "A", "is_active": true, "is_focused": true},
                    {"id": 42, "idx": 2, "output": "A", "is_active": false, "is_focused": false}
                ]
            }
        }"#;
        let second = r#"{
            "WorkspacesChanged": {
                "workspaces": [
                    {"id": 42, "idx": 1, "output": "A", "is_active": true, "is_focused": true}
                ]
            }
        }"#;
        let state = parse_workspace_event(first, &WorkspaceState::default()).expect("snapshot");
        let state = parse_workspace_event(second, &state).expect("snapshot");

        assert_eq!(state.focused_index, Some(0));
        assert!(!state.workspaces.contains_key(&41));
        assert!(state.workspaces.get(&42).expect("workspace").is_active);
    }

    #[test]
    fn ignores_replies_unknown_events_and_unknown_workspace_activations() {
        let state = WorkspaceState::default();
        assert!(parse_workspace_event(r#"{"Ok":"Handled"}"#, &state).is_none());
        assert!(parse_workspace_event(r#"{"UnknownEvent":{}}"#, &state).is_none());
        assert!(parse_workspace_event(
            r#"{"WorkspaceActivated":{"id":1,"focused":false}}"#,
            &state
        )
        .is_none());
    }
}
