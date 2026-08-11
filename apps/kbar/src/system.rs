use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

const AUDIO_REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const AUDIO_OUTPUT_REFRESH_INTERVAL: Duration = Duration::from_secs(4);
const VOLUME_SYNC_SETTLE_DELAY: Duration = Duration::from_millis(32);
const VOLUME_SYNC_RETRY_DELAY: Duration = Duration::from_millis(16);
const STATUS_REFRESH_INTERVAL: Duration = Duration::from_secs(2);
const COMMAND_TIMEOUT: Duration = Duration::from_millis(500);
const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(5);
const COMMAND_OUTPUT_DRAIN_TIMEOUT: Duration = Duration::from_millis(50);

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
    RefreshOutputs,
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AudioStatus {
    volume: VolumeStatus,
    volume_sync_token: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SlowSystemStatus {
    network_connected: bool,
    battery: Option<BatteryStatus>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum StatusUpdate {
    Audio(AudioStatus),
    SlowSystem(SlowSystemStatus),
}

/// Minimal boundary for the UI-facing audio worker.
///
/// The current implementation uses short-lived `wpctl` commands, but keeping
/// reads and mutations behind this trait leaves room for a persistent
/// PipeWire/WirePlumber client without changing the GTK side.
trait AudioBackend: Clone + Send + Sync + 'static {
    fn set_volume(&self, percent: u8);
    fn adjust_volume(&self, step: i8);
    fn toggle_mute(&self);
    fn set_default_output(&self, id: u32);
    fn read_volume_level(&self) -> Option<VolumeStatus>;
    fn read_outputs(&self) -> Option<Vec<OutputDevice>>;
}

#[derive(Clone, Copy, Debug, Default)]
struct WpctlBackend;

impl AudioBackend for WpctlBackend {
    fn set_volume(&self, percent: u8) {
        let args = vec![
            "set-volume".to_owned(),
            "@DEFAULT_AUDIO_SINK@".to_owned(),
            format!("{}%", percent.min(100)),
        ];
        let _ = run_controlled_command("wpctl", &args);
    }

    fn adjust_volume(&self, step: i8) {
        let Some(step) = volume_step_argument(step) else {
            return;
        };
        let args = vec![
            "set-volume".to_owned(),
            "@DEFAULT_AUDIO_SINK@".to_owned(),
            step,
            "--limit".to_owned(),
            "1.0".to_owned(),
        ];
        let _ = run_controlled_command("wpctl", &args);
    }

    fn toggle_mute(&self) {
        let args = vec![
            "set-mute".to_owned(),
            "@DEFAULT_AUDIO_SINK@".to_owned(),
            "toggle".to_owned(),
        ];
        let _ = run_controlled_command("wpctl", &args);
    }

    fn set_default_output(&self, id: u32) {
        let args = vec!["set-default".to_owned(), id.to_string()];
        let _ = run_controlled_command("wpctl", &args);
    }

    fn read_volume_level(&self) -> Option<VolumeStatus> {
        read_volume_level()
    }

    fn read_outputs(&self) -> Option<Vec<OutputDevice>> {
        read_outputs()
    }
}

pub fn spawn_status_worker(sender: Sender<SystemStatus>) -> Sender<VolumeAction> {
    let (action_sender, action_receiver) = mpsc::channel();
    let (update_sender, update_receiver) = mpsc::channel();

    spawn_audio_worker(WpctlBackend, action_receiver, update_sender.clone());
    spawn_system_worker(update_sender);
    thread::spawn(move || aggregate_status_updates(update_receiver, sender));

    action_sender
}

fn spawn_audio_worker<B: AudioBackend>(
    backend: B,
    action_receiver: Receiver<VolumeAction>,
    update_sender: Sender<StatusUpdate>,
) {
    thread::spawn(move || {
        let mut status = AudioStatus {
            volume: read_volume_with_backend(&backend),
            volume_sync_token: None,
        };
        let mut last_outputs_refresh = Instant::now();
        let mut last_sent = None;

        if !send_audio_status(&update_sender, &status, &mut last_sent) {
            return;
        }

        let mut pending_actions = VecDeque::new();
        loop {
            match receive_volume_action(&action_receiver, &mut pending_actions) {
                Ok(action) => {
                    execute_volume_action(&backend, &action);
                    refresh_status_after_action(&backend, &mut status, &action);
                    if matches!(
                        action,
                        VolumeAction::SetDefault(_) | VolumeAction::RefreshOutputs
                    ) {
                        last_outputs_refresh = Instant::now();
                    }
                    if !send_audio_status(&update_sender, &status, &mut last_sent) {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let mut changed = refresh_volume_level(&backend, &mut status.volume);
                    if last_outputs_refresh.elapsed() >= AUDIO_OUTPUT_REFRESH_INTERVAL {
                        changed |= refresh_outputs(&backend, &mut status.volume);
                        last_outputs_refresh = Instant::now();
                    }
                    if changed && !send_audio_status(&update_sender, &status, &mut last_sent) {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

fn spawn_system_worker(update_sender: Sender<StatusUpdate>) {
    thread::spawn(move || {
        let mut status = SlowSystemStatus {
            network_connected: read_network(),
            battery: read_battery(),
        };
        let mut last_sent = None;

        if !send_slow_system_status(&update_sender, &status, &mut last_sent) {
            return;
        }

        loop {
            thread::sleep(STATUS_REFRESH_INTERVAL);
            let next_status = SlowSystemStatus {
                network_connected: read_network(),
                battery: read_battery(),
            };
            if next_status == status {
                continue;
            }
            status = next_status;
            if !send_slow_system_status(&update_sender, &status, &mut last_sent) {
                break;
            }
        }
    });
}

fn send_audio_status(
    sender: &Sender<StatusUpdate>,
    status: &AudioStatus,
    last_sent: &mut Option<AudioStatus>,
) -> bool {
    if last_sent.as_ref() == Some(status) {
        return true;
    }
    if sender.send(StatusUpdate::Audio(status.clone())).is_err() {
        return false;
    }
    *last_sent = Some(status.clone());
    true
}

fn send_slow_system_status(
    sender: &Sender<StatusUpdate>,
    status: &SlowSystemStatus,
    last_sent: &mut Option<SlowSystemStatus>,
) -> bool {
    if last_sent.as_ref() == Some(status) {
        return true;
    }
    if sender
        .send(StatusUpdate::SlowSystem(status.clone()))
        .is_err()
    {
        return false;
    }
    *last_sent = Some(status.clone());
    true
}

fn aggregate_status_updates(receiver: Receiver<StatusUpdate>, sender: Sender<SystemStatus>) {
    let mut status = SystemStatus::default();
    let mut has_audio = false;
    let mut has_slow_system = false;
    let mut last_sent = None;

    while let Ok(first_update) = receiver.recv() {
        let mut changed = merge_status_update(
            &mut status,
            first_update,
            &mut has_audio,
            &mut has_slow_system,
        );

        while let Ok(update) = receiver.try_recv() {
            changed |=
                merge_status_update(&mut status, update, &mut has_audio, &mut has_slow_system);
        }

        if changed && has_audio && last_sent.as_ref() != Some(&status) {
            if sender.send(status.clone()).is_err() {
                break;
            }
            last_sent = Some(status.clone());
        }
    }
}

fn merge_status_update(
    status: &mut SystemStatus,
    update: StatusUpdate,
    has_audio: &mut bool,
    has_slow_system: &mut bool,
) -> bool {
    match update {
        StatusUpdate::Audio(audio) => {
            let changed = !*has_audio
                || status.volume != audio.volume
                || status.volume_sync_token != audio.volume_sync_token;
            status.volume = audio.volume;
            status.volume_sync_token = audio.volume_sync_token;
            *has_audio = true;
            changed
        }
        StatusUpdate::SlowSystem(system) => {
            let changed = !*has_slow_system
                || status.network_connected != system.network_connected
                || status.battery != system.battery;
            status.network_connected = system.network_connected;
            status.battery = system.battery;
            *has_slow_system = true;
            changed
        }
    }
}

fn receive_volume_action(
    receiver: &Receiver<VolumeAction>,
    pending_actions: &mut VecDeque<VolumeAction>,
) -> Result<VolumeAction, mpsc::RecvTimeoutError> {
    let action = match pending_actions.pop_front() {
        Some(action) => action,
        None => receiver.recv_timeout(AUDIO_REFRESH_INTERVAL)?,
    };

    let VolumeAction::Set(mut percent) = action else {
        return Ok(action);
    };

    // The UI owns the only temporal throttle. The worker only consumes a
    // contiguous run of already queued Sets, stopping before every other
    // action so ToggleMute, SetDefault and Sync retain their order.
    loop {
        let next_action = match pending_actions.pop_front() {
            Some(action) => Some(Ok(action)),
            None => match receiver.try_recv() {
                Ok(action) => Some(Ok(action)),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(())),
            },
        };

        match next_action {
            Some(Ok(VolumeAction::Set(next_percent))) => percent = next_percent,
            Some(Ok(next_action)) => {
                pending_actions.push_front(next_action);
                break;
            }
            Some(Err(())) | None => break,
        }
    }

    Ok(VolumeAction::Set(percent))
}

fn execute_volume_action<B: AudioBackend>(backend: &B, action: &VolumeAction) {
    match action {
        VolumeAction::Adjust(step) => backend.adjust_volume(*step),
        VolumeAction::Set(percent) => backend.set_volume((*percent).min(100)),
        VolumeAction::ToggleMute => backend.toggle_mute(),
        VolumeAction::SetDefault(id) => backend.set_default_output(*id),
        VolumeAction::RefreshOutputs | VolumeAction::Sync { .. } => {}
    }
}

fn refresh_status_after_action<B: AudioBackend>(
    backend: &B,
    status: &mut AudioStatus,
    action: &VolumeAction,
) {
    status.volume_sync_token = None;
    match action {
        VolumeAction::Set(percent) => {
            // Keep the slider responsive while the backend catches up. The
            // release Sync below replaces this optimistic value with a read.
            status.volume.percent = Some((*percent).min(100));
        }
        VolumeAction::Sync {
            token,
            requested_percent,
        } => {
            status.volume = read_volume_after_sync(backend, &status.volume, *requested_percent);
            status.volume_sync_token = Some(*token);
        }
        VolumeAction::SetDefault(_) => {
            refresh_volume_level(backend, &mut status.volume);
            refresh_outputs(backend, &mut status.volume);
        }
        VolumeAction::RefreshOutputs => {
            refresh_outputs(backend, &mut status.volume);
        }
        VolumeAction::Adjust(_) | VolumeAction::ToggleMute => {
            refresh_volume_level(backend, &mut status.volume);
        }
    }
}

fn read_volume_after_sync<B: AudioBackend>(
    backend: &B,
    current: &VolumeStatus,
    requested_percent: u8,
) -> VolumeStatus {
    thread::sleep(VOLUME_SYNC_SETTLE_DELAY);
    let first_read = backend.read_volume_level();
    let level = match first_read {
        Some(level) if volume_read_needs_retry(&level, requested_percent) => {
            thread::sleep(VOLUME_SYNC_RETRY_DELAY);
            backend.read_volume_level().or(Some(level))
        }
        Some(level) => Some(level),
        None => {
            thread::sleep(VOLUME_SYNC_RETRY_DELAY);
            backend.read_volume_level()
        }
    };

    let mut result = current.clone();
    if let Some(level) = level {
        result.percent = level.percent;
        result.muted = level.muted;
    }
    result
}

fn volume_read_needs_retry(status: &VolumeStatus, requested_percent: u8) -> bool {
    status
        .percent
        .map(|percent| percent.abs_diff(requested_percent) > 1)
        .unwrap_or(true)
}

fn read_volume_with_backend<B: AudioBackend>(backend: &B) -> VolumeStatus {
    let mut status = backend.read_volume_level().unwrap_or_default();
    refresh_outputs(backend, &mut status);
    status
}

fn refresh_volume_level<B: AudioBackend>(backend: &B, status: &mut VolumeStatus) -> bool {
    let Some(level) = backend.read_volume_level() else {
        return false;
    };
    let changed = status.percent != level.percent || status.muted != level.muted;
    status.percent = level.percent;
    status.muted = level.muted;
    changed
}

fn refresh_outputs<B: AudioBackend>(backend: &B, status: &mut VolumeStatus) -> bool {
    let Some(outputs) = backend.read_outputs() else {
        return false;
    };
    let previous_outputs = status.outputs.clone();
    let previous_current_output = status.current_output.clone();
    set_output_devices(status, outputs);
    previous_outputs != status.outputs || previous_current_output != status.current_output
}

fn set_output_devices(status: &mut VolumeStatus, outputs: Vec<OutputDevice>) {
    status.outputs = outputs;
    status.current_output = status
        .outputs
        .iter()
        .find(|output| output.is_default)
        .cloned();
    if status.current_output.is_none() && status.outputs.len() == 1 {
        status.current_output = status.outputs.first().cloned();
    }
}

#[cfg(test)]
fn attach_output_devices(mut status: VolumeStatus, output: &str) -> VolumeStatus {
    set_output_devices(&mut status, parse_wpctl_outputs(output));
    status
}

fn read_volume_level() -> Option<VolumeStatus> {
    let output = command_output("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
    parse_wpctl_volume(&output)
}

fn read_outputs() -> Option<Vec<OutputDevice>> {
    command_output("wpctl", &["status"]).map(|output| parse_wpctl_outputs(&output))
}

fn spawn_command(program: &str, args: &[String], stdout: Stdio) -> Option<Child> {
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

fn run_controlled_command(program: &str, args: &[String]) -> bool {
    let Some(child) = spawn_command(program, args, Stdio::null()) else {
        return false;
    };
    wait_for_command(child)
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let owned_args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    let mut child = spawn_command(program, &owned_args, Stdio::piped())?;
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

fn parse_battery_capacity(value: &str) -> Option<u8> {
    let capacity = value.trim().parse::<u16>().ok()?;
    u8::try_from(capacity.min(100)).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::{
        attach_output_devices, friendly_output_name, merge_status_update, parse_battery_capacity,
        parse_network_state, parse_wpctl_outputs, parse_wpctl_volume, receive_volume_action,
        refresh_status_after_action, run_controlled_command, AudioStatus, OutputDevice,
        SlowSystemStatus, StatusUpdate, SystemStatus, VolumeAction, VolumeStatus, WpctlBackend,
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
    fn shortens_only_universal_pipewire_output_descriptions() {
        assert_eq!(
            friendly_output_name("JBL Quantum 600 Analog Stereo"),
            "JBL Quantum 600"
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
        assert_eq!(super::volume_step_argument(5).as_deref(), Some("5%+"));
        assert_eq!(super::volume_step_argument(-5).as_deref(), Some("5%-"));
        assert_eq!(super::volume_step_argument(0), None);
    }

    #[test]
    fn coalesces_all_contiguous_slider_actions_without_waiting() {
        let (sender, receiver) = mpsc::channel();
        sender.send(VolumeAction::Set(20)).unwrap();
        sender.send(VolumeAction::Set(32)).unwrap();
        sender.send(VolumeAction::Set(57)).unwrap();
        sender.send(VolumeAction::Set(72)).unwrap();
        sender
            .send(VolumeAction::Sync {
                token: 7,
                requested_percent: 72,
            })
            .unwrap();

        let mut pending_actions = VecDeque::new();
        assert_eq!(
            receive_volume_action(&receiver, &mut pending_actions),
            Ok(VolumeAction::Set(72))
        );
        assert_eq!(
            receive_volume_action(&receiver, &mut pending_actions),
            Ok(VolumeAction::Sync {
                token: 7,
                requested_percent: 72,
            })
        );
    }

    #[test]
    fn never_crosses_non_slider_actions_when_coalescing() {
        let (sender, receiver) = mpsc::channel();
        sender.send(VolumeAction::Set(20)).unwrap();
        sender.send(VolumeAction::ToggleMute).unwrap();
        sender.send(VolumeAction::Set(32)).unwrap();
        sender.send(VolumeAction::SetDefault(52)).unwrap();
        sender.send(VolumeAction::Set(57)).unwrap();
        sender
            .send(VolumeAction::Sync {
                token: 8,
                requested_percent: 57,
            })
            .unwrap();

        let mut pending_actions = VecDeque::new();
        assert_eq!(
            receive_volume_action(&receiver, &mut pending_actions),
            Ok(VolumeAction::Set(20))
        );
        assert_eq!(
            receive_volume_action(&receiver, &mut pending_actions),
            Ok(VolumeAction::ToggleMute)
        );
        assert_eq!(
            receive_volume_action(&receiver, &mut pending_actions),
            Ok(VolumeAction::Set(32))
        );
        assert_eq!(
            receive_volume_action(&receiver, &mut pending_actions),
            Ok(VolumeAction::SetDefault(52))
        );
        assert_eq!(
            receive_volume_action(&receiver, &mut pending_actions),
            Ok(VolumeAction::Set(57))
        );
        assert_eq!(
            receive_volume_action(&receiver, &mut pending_actions),
            Ok(VolumeAction::Sync {
                token: 8,
                requested_percent: 57,
            })
        );
    }

    #[test]
    fn slider_set_updates_the_worker_snapshot_without_reading_pipewire() {
        let mut status = AudioStatus {
            volume: VolumeStatus {
                percent: Some(20),
                ..VolumeStatus::default()
            },
            volume_sync_token: None,
        };

        refresh_status_after_action(&WpctlBackend, &mut status, &VolumeAction::Set(83));

        assert_eq!(status.volume.percent, Some(83));
        assert_eq!(status.volume_sync_token, None);
    }

    #[test]
    fn identical_status_updates_are_not_reported_as_changes() {
        let audio = AudioStatus::default();
        let mut status = SystemStatus::default();
        let mut has_audio = false;
        let mut has_slow_system = false;

        assert!(merge_status_update(
            &mut status,
            StatusUpdate::Audio(audio.clone()),
            &mut has_audio,
            &mut has_slow_system,
        ));
        assert!(!merge_status_update(
            &mut status,
            StatusUpdate::Audio(audio),
            &mut has_audio,
            &mut has_slow_system,
        ));
        assert!(merge_status_update(
            &mut status,
            StatusUpdate::SlowSystem(SlowSystemStatus::default()),
            &mut has_audio,
            &mut has_slow_system,
        ));
        assert!(!merge_status_update(
            &mut status,
            StatusUpdate::SlowSystem(SlowSystemStatus::default()),
            &mut has_audio,
            &mut has_slow_system,
        ));
    }

    #[test]
    fn controlled_command_timeout_returns_without_waiting_forever() {
        let started = Instant::now();
        assert!(!run_controlled_command("sleep", &["2".to_owned()]));
        assert!(started.elapsed() < Duration::from_secs(2));
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
