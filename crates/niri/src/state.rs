//! Output-aware workspace state derived from the Niri event stream.

use serde::{Deserialize, Serialize};

use crate::protocol::Event;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Workspace {
    pub id: u64,
    #[serde(rename = "idx")]
    pub index: usize,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub is_urgent: bool,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub is_focused: bool,
    #[serde(default)]
    pub active_window_id: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceState {
    pub workspaces: Vec<Workspace>,
    pub focused_output: Option<String>,
}

impl WorkspaceState {
    pub fn focused_workspace(&self) -> Option<&Workspace> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.is_active && workspace.is_focused)
            .or_else(|| {
                self.workspaces
                    .iter()
                    .find(|workspace| workspace.is_focused)
            })
    }

    pub fn active_workspace_for(&self, output: &str) -> Option<&Workspace> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.is_active && workspace.output.as_deref() == Some(output))
    }

    pub fn active_index_for(&self, output: &str) -> Option<usize> {
        self.active_workspace_for(output)
            .map(|workspace| workspace.index)
    }

    pub fn apply_event(&self, event: Event) -> Option<Self> {
        match event {
            Event::WorkspacesChanged { workspaces } => Some(Self::from_workspaces(workspaces)),
            Event::WorkspaceActivated { id, focused } => {
                let workspace = self
                    .workspaces
                    .iter()
                    .find(|workspace| workspace.id == id)?;
                let output = workspace.output.clone();
                let mut next = self.clone();

                for workspace in &mut next.workspaces {
                    if workspace.output == output {
                        workspace.is_active = workspace.id == id;
                    }
                    if focused {
                        workspace.is_focused = workspace.id == id;
                    }
                }
                next.refresh_focused_output();
                Some(next)
            }
            Event::WorkspaceUrgencyChanged { id, urgent } => {
                let index = self
                    .workspaces
                    .iter()
                    .position(|workspace| workspace.id == id)?;
                let mut next = self.clone();
                next.workspaces[index].is_urgent = urgent;
                Some(next)
            }
            Event::Other => None,
        }
    }

    fn from_workspaces(workspaces: Vec<Workspace>) -> Self {
        let mut state = Self {
            workspaces,
            focused_output: None,
        };
        state.refresh_focused_output();
        state
    }

    fn refresh_focused_output(&mut self) {
        self.focused_output = self
            .focused_workspace()
            .and_then(|workspace| workspace.output.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::{Workspace, WorkspaceState};
    use crate::protocol::{parse_event, Event};

    fn workspace(id: u64, index: usize, output: &str, active: bool, focused: bool) -> Workspace {
        Workspace {
            id,
            index,
            name: None,
            output: Some(output.to_owned()),
            is_urgent: false,
            is_active: active,
            is_focused: focused,
            active_window_id: None,
        }
    }

    #[test]
    fn snapshot_replaces_the_previous_state_and_tracks_focus_output() {
        let first = WorkspaceState::default()
            .apply_event(Event::WorkspacesChanged {
                workspaces: vec![
                    workspace(1, 1, "A", true, true),
                    workspace(2, 2, "A", false, false),
                ],
            })
            .unwrap();
        let second = first
            .apply_event(Event::WorkspacesChanged {
                workspaces: vec![workspace(2, 1, "B", true, true)],
            })
            .unwrap();

        assert_eq!(second.workspaces.len(), 1);
        assert_eq!(second.focused_output.as_deref(), Some("B"));
        assert_eq!(second.active_index_for("B"), Some(1));
        assert!(second.active_workspace_for("A").is_none());
    }

    #[test]
    fn activation_is_scoped_to_the_workspace_output() {
        let state = WorkspaceState::default()
            .apply_event(Event::WorkspacesChanged {
                workspaces: vec![
                    workspace(1, 1, "A", true, true),
                    workspace(2, 2, "A", false, false),
                    workspace(3, 1, "B", true, false),
                ],
            })
            .unwrap();
        let state = state
            .apply_event(Event::WorkspaceActivated {
                id: 2,
                focused: false,
            })
            .unwrap();

        assert_eq!(state.active_index_for("A"), Some(2));
        assert_eq!(state.active_index_for("B"), Some(1));
        assert_eq!(
            state.focused_workspace().map(|workspace| workspace.id),
            Some(1)
        );
    }

    #[test]
    fn typed_snapshot_parses_optional_new_fields_with_defaults() {
        let event = parse_event(
            r#"{"WorkspacesChanged":{"workspaces":[{"id":11,"idx":1,"output":"A","is_active":true,"is_focused":true}]}}"#,
        )
        .unwrap();
        let state = WorkspaceState::default().apply_event(event).unwrap();

        assert_eq!(state.focused_workspace().unwrap().index, 1);
        assert!(!state.focused_workspace().unwrap().is_urgent);
    }

    #[test]
    fn unknown_events_do_not_change_state() {
        let state = WorkspaceState::default();
        assert!(state.apply_event(Event::Other).is_none());
    }
}
