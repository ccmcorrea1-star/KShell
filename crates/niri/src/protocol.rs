//! The small, typed subset of Niri's JSON protocol used by KShell.

use serde::de::{self, IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::state::Workspace;

const EVENT_STREAM_REQUEST: Request = Request::EventStream;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    WorkspacesChanged { workspaces: Vec<Workspace> },
    WorkspaceActivated { id: u64, focused: bool },
    WorkspaceUrgencyChanged { id: u64, urgent: bool },
    Other,
}

#[derive(Deserialize)]
struct WorkspacesChangedPayload {
    workspaces: Vec<Workspace>,
}

#[derive(Deserialize)]
struct WorkspaceActivatedPayload {
    id: u64,
    focused: bool,
}

#[derive(Deserialize)]
struct WorkspaceUrgencyChangedPayload {
    id: u64,
    urgent: bool,
}

impl<'de> Deserialize<'de> for Event {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EventVisitor;

        impl<'de> Visitor<'de> for EventVisitor {
            type Value = Event;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a Niri event object")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let Some(name) = map.next_key::<String>()? else {
                    return Err(de::Error::custom("Niri event object is empty"));
                };

                let event = match name.as_str() {
                    "WorkspacesChanged" => {
                        let payload: WorkspacesChangedPayload = map.next_value()?;
                        Event::WorkspacesChanged {
                            workspaces: payload.workspaces,
                        }
                    }
                    "WorkspaceActivated" => {
                        let payload: WorkspaceActivatedPayload = map.next_value()?;
                        Event::WorkspaceActivated {
                            id: payload.id,
                            focused: payload.focused,
                        }
                    }
                    "WorkspaceUrgencyChanged" => {
                        let payload: WorkspaceUrgencyChangedPayload = map.next_value()?;
                        Event::WorkspaceUrgencyChanged {
                            id: payload.id,
                            urgent: payload.urgent,
                        }
                    }
                    _ => {
                        let _: IgnoredAny = map.next_value()?;
                        Event::Other
                    }
                };

                while map.next_key::<IgnoredAny>()?.is_some() {
                    let _: IgnoredAny = map.next_value()?;
                }

                Ok(event)
            }
        }

        deserializer.deserialize_map(EventVisitor)
    }
}

#[derive(Clone, Debug, Serialize)]
pub enum Request {
    EventStream,
    Action(Action),
}

#[derive(Clone, Debug, Serialize)]
pub enum Action {
    FocusWorkspace { reference: WorkspaceReference },
}

#[derive(Clone, Debug, Serialize)]
pub enum WorkspaceReference {
    Index(usize),
    Id(u64),
    Name(String),
}

pub fn parse_event(line: &str) -> Result<Event, serde_json::Error> {
    serde_json::from_str(line)
}

pub fn event_stream_request() -> Result<Vec<u8>, serde_json::Error> {
    encode_request(&EVENT_STREAM_REQUEST)
}

pub fn focus_workspace_request(index: usize) -> Result<Vec<u8>, serde_json::Error> {
    focus_workspace_reference_request(WorkspaceReference::Index(index))
}

pub fn focus_workspace_reference_request(
    reference: WorkspaceReference,
) -> Result<Vec<u8>, serde_json::Error> {
    encode_request(&Request::Action(Action::FocusWorkspace { reference }))
}

fn encode_request(request: &Request) -> Result<Vec<u8>, serde_json::Error> {
    let mut encoded = serde_json::to_vec(request)?;
    encoded.push(b'\n');
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::{focus_workspace_request, parse_event, Event};

    #[test]
    fn encodes_a_typed_focus_workspace_request() {
        assert_eq!(
            focus_workspace_request(3).unwrap(),
            b"{\"Action\":{\"FocusWorkspace\":{\"reference\":{\"Index\":3}}}}\n"
        );
    }

    #[test]
    fn parses_typed_workspace_events() {
        let event = parse_event(r#"{"WorkspaceActivated":{"id":12,"focused":true}}"#).unwrap();

        assert_eq!(
            event,
            Event::WorkspaceActivated {
                id: 12,
                focused: true,
            }
        );
    }

    #[test]
    fn accepts_unknown_events_and_replies_without_json_value_parsing() {
        assert_eq!(
            parse_event(r#"{"SomeFutureEvent":{}}"#).unwrap(),
            Event::Other
        );
        assert_eq!(parse_event(r#"{"Ok":"Handled"}"#).unwrap(), Event::Other);
    }
}
