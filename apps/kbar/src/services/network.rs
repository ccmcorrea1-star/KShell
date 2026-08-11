//! Network status source. The command boundary can later be replaced by
//! NetworkManager events without changing the status widget.

use super::command;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NetworkStatus {
    pub connected: bool,
}

pub fn read() -> NetworkStatus {
    if let Some(output) = command::output("nmcli", &["-t", "-f", "STATE", "general"]) {
        return NetworkStatus {
            connected: parse_state(&output),
        };
    }

    NetworkStatus {
        connected: command::output("ip", &["route", "show", "default"])
            .is_some_and(|output| !output.trim().is_empty()),
    }
}

fn parse_state(output: &str) -> bool {
    output.trim().to_ascii_lowercase().starts_with("connected")
}

#[cfg(test)]
mod tests {
    use super::parse_state;

    #[test]
    fn recognizes_connected_network_state() {
        assert!(parse_state("connected (global)\n"));
        assert!(!parse_state("disconnected\n"));
    }
}
