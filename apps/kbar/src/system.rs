use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

const VOLUME_SET_BATCH_WINDOW: Duration = Duration::from_millis(40);
const VOLUME_SYNC_SETTLE_DELAY: Duration = Duration::from_millis(32);
const VOLUME_SYNC_RETRY_DELAY: Duration = Duration::from_millis(16);
const VOLUME_SYNC_TIMEOUT: Duration = Duration::from_millis(160);
const AUDIO_REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const STATUS_REFRESH_INTERVAL: Duration = Duration::from_secs(2);
const COMMAND_TIMEOUT: Duration = Duration::from_millis(500);
const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VolumeStatus {
    pub percent: Option<u8>,
    pub muted: bool,
    pub current_output: Option<OutputDevice>,
    pub outputs: Vec<OutputDevice>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputDevice {
    pub id: u32,
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VolumeAction {
    Adjust(i8),
    Set(u8),
    ToggleMute,
    SetDefault(u32),
    Sync { token: u64, requested_percent: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatteryStatus {
    pub percent: u8,
    pub charging: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SystemStatus {
    pub volume: VolumeStatus,
    pub volume_sync_token: Option<u64>,
    pub network_connected: bool,
    pub battery: Option<BatteryStatus>,
}

pub fn spawn_status_worker(sender: Sender<SystemStatus>) -> Sender<VolumeAction> {
    let (action_sender, action_receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut pending_actions = VecDeque::new();
        let mut status = read_status();
        let mut last_system_refresh = Instant::now();
        if sender.send(status.clone()).is_err() {
            return;
        }

        loop {
            match receive_volume_action(&action_receiver, &mut pending_actions) {
                Ok(action) => {
                    execute_volume_action(action.clone());
                    refresh_status_after_action(&mut status, &action);
                    if sender.send(status.clone()).is_err() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    status.volume = read_volume();
                    if last_system_refresh.elapsed() >= STATUS_REFRESH_INTERVAL {
                        status.network_connected = read_network();
                        status.battery = read_battery();
                        last_system_refresh = Instant::now();
                    }
                    if sender.send(status.clone()).is_err() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
    action_sender
}

fn receive_volume_action(
    receiver: &Receiver<VolumeAction>,
    pending_actions: &mut VecDeque<VolumeAction>,
) -> Result<VolumeAction, mpsc::RecvTimeoutError> {
    let action = match pending_actions.pop_front() {
        Some(action) => action,
        None => receiver.recv_timeout(AUDIO_REFRESH_INTERVAL)?,
    };

    match action {
        VolumeAction::Set(mut percent) => {
            let deadline = Instant::now() + VOLUME_SET_BATCH_WINDOW;
            let mut encountered_non_set = false;
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }

                match receiver.recv_timeout(remaining) {
                    Ok(VolumeAction::Set(next_percent)) => percent = next_percent,
                    Ok(next_action) => {
                        pending_actions.push_back(next_action);
                        encountered_non_set = true;
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout)
                    | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }

            // If wpctl was busy executing the previous update, several
            // throttled slider values may already be queued. Keep only the
            // newest Set before the next non-slider action, usually Sync, so
            // PipeWire does not replay stale positions after release.
            if !encountered_non_set {
                loop {
                    match receiver.try_recv() {
                        Ok(VolumeAction::Set(next_percent)) => percent = next_percent,
                        Ok(next_action) => {
                            pending_actions.push_back(next_action);
                            break;
                        }
                        Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => {
                            break;
                        }
                    }
                }
            }
            Ok(VolumeAction::Set(percent))
        }
        action => Ok(action),
    }
}

fn refresh_status_after_action(status: &mut SystemStatus, action: &VolumeAction) {
    status.volume_sync_token = None;
    match action {
        VolumeAction::Set(percent) => {
            // Keep the slider responsive while PipeWire catches up. The explicit
            // Sync action sent on release replaces this optimistic value with
            // the state actually reported by PipeWire.
            status.volume.percent = Some((*percent).min(100));
        }
        VolumeAction::Sync {
            token,
            requested_percent,
        } => {
            let volume = read_volume_after_sync(*requested_percent);
            status.volume = volume;
            status.volume_sync_token = Some(*token);
        }
        VolumeAction::SetDefault(_) => {
            status.volume = read_volume();
        }
        VolumeAction::Adjust(_) | VolumeAction::ToggleMute => {
            if let Some(volume) = read_volume_level() {
                status.volume.percent = volume.percent;
                status.volume.muted = volume.muted;
            }
        }
    }
}

fn read_volume_after_sync(requested_percent: u8) -> VolumeStatus {
    thread::sleep(VOLUME_SYNC_SETTLE_DELAY);
    let deadline = Instant::now() + VOLUME_SYNC_TIMEOUT;
    let mut percent = read_volume_level().and_then(|volume| volume.percent);
    while percent != Some(requested_percent) && Instant::now() < deadline {
        thread::sleep(VOLUME_SYNC_RETRY_DELAY);
        percent = read_volume_level().and_then(|volume| volume.percent);
    }
    read_volume()
}

fn read_status() -> SystemStatus {
    SystemStatus {
        volume: read_volume(),
        volume_sync_token: None,
        network_connected: read_network(),
        battery: read_battery(),
    }
}

fn read_volume() -> VolumeStatus {
    let mut status = read_volume_level().unwrap_or_default();

    if let Some(output) = command_output("wpctl", &["status"]) {
        status = attach_output_devices(status, &output);
    }

    status
}

fn attach_output_devices(mut status: VolumeStatus, output: &str) -> VolumeStatus {
    status.outputs = parse_wpctl_outputs(output);
    status.current_output = status
        .outputs
        .iter()
        .find(|output| output.is_default)
        .cloned();
    if status.current_output.is_none() && status.outputs.len() == 1 {
        status.current_output = status.outputs.first().cloned();
    }
    status
}

fn read_volume_level() -> Option<VolumeStatus> {
    let output = command_output("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
    parse_wpctl_volume(&output)
}

fn execute_volume_action(action: VolumeAction) {
    let args = match action {
        VolumeAction::Adjust(step) => {
            let Some(step) = volume_step_argument(step) else {
                return;
            };
            vec![
                "set-volume".to_owned(),
                "@DEFAULT_AUDIO_SINK@".to_owned(),
                step,
                "--limit".to_owned(),
                "1.0".to_owned(),
            ]
        }
        VolumeAction::Set(percent) => vec![
            "set-volume".to_owned(),
            "@DEFAULT_AUDIO_SINK@".to_owned(),
            format!("{}%", percent.min(100)),
        ],
        VolumeAction::ToggleMute => vec![
            "set-mute".to_owned(),
            "@DEFAULT_AUDIO_SINK@".to_owned(),
            "toggle".to_owned(),
        ],
        VolumeAction::SetDefault(id) => vec!["set-default".to_owned(), id.to_string()],
        VolumeAction::Sync { .. } => return,
    };

    let _ = Command::new("wpctl").args(args).env("LC_ALL", "C").status();
}

fn read_network() -> bool {
    if let Some(output) = command_output("nmcli", &["-t", "-f", "STATE", "general"]) {
        return parse_network_state(&output);
    }

    command_output("ip", &["route", "show", "default"])
        .is_some_and(|output| !output.trim().is_empty())
}

fn read_battery() -> Option<BatteryStatus> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    let mut capacities = Vec::new();
    let mut charging = false;

    for entry in entries.flatten() {
        let path = entry.path();
        if !fs::read_to_string(path.join("type"))
            .map(|kind| kind.trim().eq_ignore_ascii_case("battery"))
            .unwrap_or(false)
        {
            continue;
        }

        let Some(capacity) = fs::read_to_string(path.join("capacity"))
            .ok()
            .and_then(|value| parse_battery_capacity(&value))
        else {
            continue;
        };
        capacities.push(capacity);
        charging |= fs::read_to_string(path.join("status"))
            .map(|status| {
                matches!(
                    status.trim().to_ascii_lowercase().as_str(),
                    "charging" | "full"
                )
            })
            .unwrap_or(false);
    }

    if capacities.is_empty() {
        return None;
    }

    let percent = capacities
        .iter()
        .map(|&value| u32::from(value))
        .sum::<u32>()
        / u32::try_from(capacities.len()).ok()?;
    Some(BatteryStatus {
        percent: u8::try_from(percent).unwrap_or(100),
        charging,
    })
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let stdout_reader = thread::spawn(move || {
        let mut output = String::new();
        stdout.read_to_string(&mut output).map(|_| output)
    });

    let deadline = Instant::now() + COMMAND_TIMEOUT;
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(COMMAND_POLL_INTERVAL),
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };

    let output = stdout_reader.join().ok()?.ok()?;
    success.then_some(output)
}

fn parse_wpctl_volume(output: &str) -> Option<VolumeStatus> {
    let volume_values = output.lines().find_map(|line| {
        let (label, values) = line.split_once(':')?;
        label
            .trim()
            .eq_ignore_ascii_case("volume")
            .then_some(values)
    })?;
    let value = volume_values
        .split_whitespace()
        .find_map(|part| part.parse::<f32>().ok())?;
    if !value.is_finite() {
        return None;
    }

    Some(VolumeStatus {
        percent: Some((value.clamp(0.0, 1.0) * 100.0).round() as u8),
        muted: volume_values
            .split_whitespace()
            .any(|part| part.trim_matches(['[', ']']).eq_ignore_ascii_case("muted")),
        current_output: None,
        outputs: Vec::new(),
    })
}

fn parse_wpctl_outputs(output: &str) -> Vec<OutputDevice> {
    let mut in_sinks = false;
    let mut outputs: Vec<OutputDevice> = Vec::new();

    for line in output.lines() {
        let section = line.trim_start_matches(|character: char| {
            character.is_whitespace() || matches!(character, '│' | '├' | '└' | '─')
        });

        if section.starts_with("Sinks:") {
            in_sinks = true;
            continue;
        }
        if in_sinks
            && [
                "Sources:",
                "Filters:",
                "Streams:",
                "Devices:",
                "Sink endpoints:",
                "Source endpoints:",
            ]
            .iter()
            .any(|next_section| section.starts_with(next_section))
        {
            break;
        }
        if !in_sinks {
            continue;
        }

        let Some(output) = parse_wpctl_output_line(line) else {
            continue;
        };
        if outputs.iter().all(|existing| existing.id != output.id) {
            outputs.push(output);
        }
    }

    outputs
}

fn parse_wpctl_output_line(line: &str) -> Option<OutputDevice> {
    let entry = line.trim_start_matches(|character: char| {
        character.is_whitespace() || matches!(character, '│' | '├' | '└' | '─')
    });
    let is_default = entry.starts_with('*');
    let entry = entry.strip_prefix('*').unwrap_or(entry).trim_start();
    let (id, name) = entry.split_once('.')?;
    let id = id.trim().parse().ok()?;
    let name = name
        .split_once(" [vol:")
        .map(|(name, _)| name)
        .unwrap_or(name)
        .trim()
        .to_owned();
    let name = friendly_output_name(&name);

    if name.is_empty() {
        return None;
    }

    Some(OutputDevice {
        id,
        name,
        is_default,
    })
}

fn friendly_output_name(name: &str) -> String {
    let name = name.trim();
    let normalized = name.to_lowercase();

    if normalized.contains("quantum 600") {
        return "Quantum 600".to_owned();
    }
    if normalized.contains("hdmi") {
        return "HDMI".to_owned();
    }
    if normalized.contains("displayport") {
        return "DisplayPort".to_owned();
    }
    if normalized.contains("iec958")
        || normalized.contains("spdif")
        || normalized.contains("s/pdif")
    {
        return "S/PDIF".to_owned();
    }

    [
        " Analog Stereo",
        " Estéreo analógico",
        " Stereo",
        " Estéreo",
    ]
    .iter()
    .find_map(|suffix| name.strip_suffix(suffix))
    .unwrap_or(name)
    .trim()
    .to_owned()
}

fn volume_step_argument(step: i8) -> Option<String> {
    let step = i16::from(step);
    if step == 0 {
        return None;
    }
    Some(format!(
        "{}%{}",
        step.abs(),
        if step > 0 { "+" } else { "-" }
    ))
}

fn parse_network_state(output: &str) -> bool {
    output.trim().to_ascii_lowercase().starts_with("connected")
}

fn parse_battery_capacity(value: &str) -> Option<u8> {
    let capacity = value.trim().parse::<u16>().ok()?;
    u8::try_from(capacity.min(100)).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::mpsc;

    use super::{
        attach_output_devices, friendly_output_name, parse_battery_capacity, parse_network_state,
        parse_wpctl_outputs, parse_wpctl_volume, refresh_status_after_action, volume_step_argument,
        OutputDevice, SystemStatus, VolumeAction, VolumeStatus,
    };

    #[test]
    fn parses_pipewire_volume_and_mute_state() {
        assert_eq!(
            parse_wpctl_volume("Volume: 0.42 [MUTED]"),
            Some(VolumeStatus {
                percent: Some(42),
                muted: true,
                current_output: None,
                outputs: Vec::new(),
            })
        );
    }

    #[test]
    fn reads_volume_from_the_volume_field_instead_of_an_unrelated_number() {
        assert_eq!(
            parse_wpctl_volume("Object: 52\nVolume: 0.42 [MUTED]"),
            Some(VolumeStatus {
                percent: Some(42),
                muted: true,
                current_output: None,
                outputs: Vec::new(),
            })
        );
    }

    #[test]
    fn parses_pipewire_output_list_and_default() {
        assert_eq!(
            parse_wpctl_outputs(
                "Audio\n ├─ Sinks:\n │  *   52. Built-in Audio Analog Stereo [vol: 0.80]\n │      53. HDMI / DisplayPort [vol: 0.35]\n ├─ Sources:\n │      54. Built-in Audio Analog Stereo"
            ),
            vec![
                OutputDevice {
                    id: 52,
                    name: "Built-in Audio".to_owned(),
                    is_default: true,
                },
                OutputDevice {
                    id: 53,
                    name: "HDMI".to_owned(),
                    is_default: false,
                },
            ]
        );
    }

    #[test]
    fn stops_output_parsing_at_endpoint_sections() {
        assert_eq!(
            parse_wpctl_outputs(
                "Audio\n ├─ Sinks:\n │  *   52. Built-in Audio Analog Stereo [vol: 0.80]\n ├─ Sink endpoints:\n │      99. Endpoint that is not a sink"
            ),
            vec![OutputDevice {
                id: 52,
                name: "Built-in Audio".to_owned(),
                is_default: true,
            }]
        );
    }

    #[test]
    fn rejects_sink_lines_without_a_numeric_id_prefix() {
        assert!(super::parse_wpctl_output_line(" │      not a sink entry").is_none());
    }

    #[test]
    fn keeps_output_devices_when_the_default_volume_is_unavailable() {
        let status = attach_output_devices(
            VolumeStatus::default(),
            "Audio\n ├─ Sinks:\n │      52. Built-in Audio Analog Stereo [vol: 0.80]\n ├─ Sources:",
        );

        assert_eq!(status.percent, None);
        assert_eq!(
            status.current_output.as_ref().map(|output| output.id),
            Some(52)
        );
        assert_eq!(status.outputs.len(), 1);
    }

    #[test]
    fn shortens_common_pipewire_output_descriptions() {
        assert_eq!(
            friendly_output_name("JBL Quantum 600 Analog Stereo"),
            "Quantum 600"
        );
        assert_eq!(
            friendly_output_name("Quantum 600 Estéreo analógico"),
            "Quantum 600"
        );
        assert_eq!(friendly_output_name("HDMI / DisplayPort"), "HDMI");
        assert_eq!(friendly_output_name("S/PDIF Digital Stereo"), "S/PDIF");
        assert_eq!(
            friendly_output_name("Starship/Matisse HD Audio Controller Estéreo digital (IEC958)"),
            "S/PDIF"
        );
        assert_eq!(friendly_output_name("Built-in Audio"), "Built-in Audio");
    }

    #[test]
    fn builds_pipewire_volume_step_arguments() {
        assert_eq!(volume_step_argument(5).as_deref(), Some("5%+"));
        assert_eq!(volume_step_argument(-5).as_deref(), Some("5%-"));
        assert_eq!(volume_step_argument(0), None);
    }

    #[test]
    fn coalesces_slider_actions_without_reordering_other_actions() {
        let (sender, receiver) = mpsc::channel();
        sender.send(super::VolumeAction::Set(20)).unwrap();
        sender.send(super::VolumeAction::Set(40)).unwrap();
        sender.send(super::VolumeAction::ToggleMute).unwrap();
        sender.send(super::VolumeAction::Set(60)).unwrap();

        let mut pending_actions = VecDeque::new();
        assert_eq!(
            super::receive_volume_action(&receiver, &mut pending_actions),
            Ok(super::VolumeAction::Set(40))
        );
        assert_eq!(
            super::receive_volume_action(&receiver, &mut pending_actions),
            Ok(super::VolumeAction::ToggleMute)
        );
        assert_eq!(
            super::receive_volume_action(&receiver, &mut pending_actions),
            Ok(super::VolumeAction::Set(60))
        );
    }

    #[test]
    fn keeps_release_sync_after_the_last_coalesced_slider_value() {
        let (sender, receiver) = mpsc::channel();
        sender.send(VolumeAction::Set(80)).unwrap();
        sender
            .send(VolumeAction::Sync {
                token: 7,
                requested_percent: 80,
            })
            .unwrap();

        let mut pending_actions = VecDeque::new();
        assert_eq!(
            super::receive_volume_action(&receiver, &mut pending_actions),
            Ok(VolumeAction::Set(80))
        );
        assert_eq!(
            super::receive_volume_action(&receiver, &mut pending_actions),
            Ok(VolumeAction::Sync {
                token: 7,
                requested_percent: 80,
            })
        );
    }

    #[test]
    fn slider_set_updates_the_worker_snapshot_without_reading_pipewire() {
        let mut status = SystemStatus::default();
        status.volume.percent = Some(20);

        refresh_status_after_action(&mut status, &VolumeAction::Set(83));

        assert_eq!(status.volume.percent, Some(83));
        assert_eq!(status.volume_sync_token, None);
    }

    #[test]
    fn recognizes_network_state_without_showing_a_label() {
        assert!(parse_network_state("connected (global)\n"));
        assert!(!parse_network_state("disconnected\n"));
    }

    #[test]
    fn clamps_battery_capacity_to_a_percentage() {
        assert_eq!(parse_battery_capacity("87\n"), Some(87));
        assert_eq!(parse_battery_capacity("120"), Some(100));
        assert_eq!(parse_battery_capacity("unknown"), None);
    }
}
