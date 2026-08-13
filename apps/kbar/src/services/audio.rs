//! Audio worker and PipeWire/WirePlumber backend.

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use super::command;
use super::StatusUpdate;

const AUDIO_REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const AUDIO_OUTPUT_REFRESH_INTERVAL: Duration = Duration::from_secs(4);
const VOLUME_SYNC_SETTLE_DELAY: Duration = Duration::from_millis(32);
const VOLUME_SYNC_RETRY_DELAY: Duration = Duration::from_millis(16);

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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AudioStatus {
    pub volume: VolumeStatus,
    pub volume_sync_token: Option<u64>,
}

/// Minimal boundary for the UI-facing audio worker.
///
/// The current implementation uses short-lived `wpctl` commands, but keeping
/// reads and mutations behind this trait leaves room for a persistent
/// PipeWire/WirePlumber client without changing the GTK side.
pub trait AudioBackend: Clone + Send + Sync + 'static {
    fn set_volume(&self, percent: u8);
    fn adjust_volume(&self, step: i8);
    fn toggle_mute(&self);
    fn set_default_output(&self, id: u32);
    fn read_volume_level(&self) -> Option<VolumeStatus>;
    fn read_outputs(&self) -> Option<Vec<OutputDevice>>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WpctlBackend;

impl AudioBackend for WpctlBackend {
    fn set_volume(&self, percent: u8) {
        let args = vec![
            "set-volume".to_owned(),
            "@DEFAULT_AUDIO_SINK@".to_owned(),
            format!("{}%", percent.min(100)),
        ];
        let _ = command::run_controlled("wpctl", &args);
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
        let _ = command::run_controlled("wpctl", &args);
    }

    fn toggle_mute(&self) {
        let args = vec![
            "set-mute".to_owned(),
            "@DEFAULT_AUDIO_SINK@".to_owned(),
            "toggle".to_owned(),
        ];
        let _ = command::run_controlled("wpctl", &args);
    }

    fn set_default_output(&self, id: u32) {
        let args = vec!["set-default".to_owned(), id.to_string()];
        let _ = command::run_controlled("wpctl", &args);
    }

    fn read_volume_level(&self) -> Option<VolumeStatus> {
        read_volume_level()
    }

    fn read_outputs(&self) -> Option<Vec<OutputDevice>> {
        read_outputs()
    }
}

pub fn spawn_audio_worker<B: AudioBackend>(
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
        let mut pending_sync = None;

        if !send_audio_status(&update_sender, &mut status, &mut last_sent) {
            return;
        }

        let mut pending_actions = VecDeque::new();
        loop {
            match receive_volume_action(&action_receiver, &mut pending_actions) {
                Ok(action) => {
                    execute_volume_action(&backend, &action);
                    let unconfirmed_sync =
                        refresh_status_after_action(&backend, &mut status, &action);
                    match &action {
                        VolumeAction::Set(_)
                        | VolumeAction::Adjust(_)
                        | VolumeAction::SetDefault(_) => pending_sync = None,
                        VolumeAction::Sync { .. } => pending_sync = unconfirmed_sync,
                        VolumeAction::ToggleMute | VolumeAction::RefreshOutputs => {}
                    }
                    if matches!(
                        action,
                        VolumeAction::SetDefault(_) | VolumeAction::RefreshOutputs
                    ) {
                        last_outputs_refresh = Instant::now();
                    }
                    if !send_audio_status(&update_sender, &mut status, &mut last_sent) {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let (mut changed, valid_volume_read) =
                        refresh_volume_level_with_validity(&backend, &mut status.volume);
                    if let Some((token, requested_percent)) = pending_sync {
                        let confirmed = valid_volume_read
                            && status
                                .volume
                                .percent
                                .is_some_and(|percent| percent.abs_diff(requested_percent) <= 1);
                        if confirmed {
                            status.volume_sync_token = Some(token);
                            pending_sync = None;
                            changed = true;
                        }
                    }
                    if last_outputs_refresh.elapsed() >= AUDIO_OUTPUT_REFRESH_INTERVAL {
                        changed |= refresh_outputs(&backend, &mut status.volume);
                        last_outputs_refresh = Instant::now();
                    }
                    if changed && !send_audio_status(&update_sender, &mut status, &mut last_sent) {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

fn send_audio_status(
    sender: &Sender<StatusUpdate>,
    status: &mut AudioStatus,
    last_sent: &mut Option<AudioStatus>,
) -> bool {
    if last_sent.as_ref() == Some(status) {
        return true;
    }
    if sender.send(StatusUpdate::Audio(status.clone())).is_err() {
        return false;
    }
    status.volume_sync_token = None;
    *last_sent = Some(status.clone());
    true
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
) -> Option<(u64, u8)> {
    status.volume_sync_token = None;
    match action {
        VolumeAction::Set(percent) => {
            // Keep the slider responsive while the backend catches up. The
            // release Sync below replaces this optimistic value with a read.
            status.volume.percent = Some((*percent).min(100));
            None
        }
        VolumeAction::Sync {
            token,
            requested_percent,
        } => {
            let (volume, confirmed) =
                read_volume_after_sync(backend, &status.volume, *requested_percent);
            status.volume = volume;
            if confirmed {
                status.volume_sync_token = Some(*token);
                None
            } else {
                Some((*token, *requested_percent))
            }
        }
        VolumeAction::SetDefault(_) => {
            refresh_volume_level(backend, &mut status.volume);
            refresh_outputs(backend, &mut status.volume);
            None
        }
        VolumeAction::RefreshOutputs => {
            refresh_outputs(backend, &mut status.volume);
            None
        }
        VolumeAction::Adjust(_) | VolumeAction::ToggleMute => {
            refresh_volume_level(backend, &mut status.volume);
            None
        }
    }
}

fn read_volume_after_sync<B: AudioBackend>(
    backend: &B,
    current: &VolumeStatus,
    requested_percent: u8,
) -> (VolumeStatus, bool) {
    thread::sleep(VOLUME_SYNC_SETTLE_DELAY);
    let first_read = backend.read_volume_level();
    let (level, confirmed) = match first_read {
        Some(level) if volume_read_needs_retry(&level, requested_percent) => {
            thread::sleep(VOLUME_SYNC_RETRY_DELAY);
            match backend.read_volume_level() {
                Some(level) => {
                    let confirmed = !volume_read_needs_retry(&level, requested_percent);
                    (Some(level), confirmed)
                }
                None => (Some(level), false),
            }
        }
        Some(level) => (Some(level), true),
        None => {
            thread::sleep(VOLUME_SYNC_RETRY_DELAY);
            match backend.read_volume_level() {
                Some(level) => {
                    let confirmed = !volume_read_needs_retry(&level, requested_percent);
                    (Some(level), confirmed)
                }
                None => (None, false),
            }
        }
    };

    let mut result = current.clone();
    if let Some(level) = level {
        result.percent = level.percent;
        result.muted = level.muted;
    } else {
        result.percent = None;
    }
    (result, confirmed)
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
    refresh_volume_level_with_validity(backend, status).0
}

fn refresh_volume_level_with_validity<B: AudioBackend>(
    backend: &B,
    status: &mut VolumeStatus,
) -> (bool, bool) {
    let Some(level) = backend.read_volume_level() else {
        return (false, false);
    };
    let changed = status.percent != level.percent || status.muted != level.muted;
    status.percent = level.percent;
    status.muted = level.muted;
    (changed, true)
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
    let output = command::output("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
    parse_wpctl_volume(&output)
}

fn read_outputs() -> Option<Vec<OutputDevice>> {
    command::output("wpctl", &["status"]).map(|output| parse_wpctl_outputs(&output))
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

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{mpsc, Arc, Mutex};

    use super::{
        attach_output_devices, friendly_output_name, parse_wpctl_outputs, parse_wpctl_volume,
        receive_volume_action, refresh_status_after_action, send_audio_status, AudioBackend,
        AudioStatus, OutputDevice, StatusUpdate, VolumeAction, VolumeStatus, WpctlBackend,
    };

    #[derive(Clone, Default)]
    struct ReadSequenceBackend {
        reads: Arc<Mutex<VecDeque<Option<VolumeStatus>>>>,
    }

    impl AudioBackend for ReadSequenceBackend {
        fn set_volume(&self, _percent: u8) {}

        fn adjust_volume(&self, _step: i8) {}

        fn toggle_mute(&self) {}

        fn set_default_output(&self, _id: u32) {}

        fn read_volume_level(&self) -> Option<VolumeStatus> {
            self.reads.lock().expect("read sequence lock").pop_front()?
        }

        fn read_outputs(&self) -> Option<Vec<OutputDevice>> {
            None
        }
    }

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
    fn an_unavailable_sync_keeps_confirmation_pending() {
        let backend = ReadSequenceBackend {
            reads: Arc::new(Mutex::new(VecDeque::from([None, None]))),
        };
        let mut status = AudioStatus {
            volume: VolumeStatus {
                percent: Some(83),
                ..VolumeStatus::default()
            },
            volume_sync_token: None,
        };

        let pending = refresh_status_after_action(
            &backend,
            &mut status,
            &VolumeAction::Sync {
                token: 9,
                requested_percent: 83,
            },
        );

        assert_eq!(pending, Some((9, 83)));
        assert_eq!(status.volume.percent, None);
        assert_eq!(status.volume_sync_token, None);
    }

    #[test]
    fn confirmed_tokens_are_one_shot_status_metadata() {
        let (sender, receiver) = mpsc::channel();
        let mut status = AudioStatus {
            volume_sync_token: Some(11),
            ..AudioStatus::default()
        };
        let mut last_sent = None;

        assert!(send_audio_status(&sender, &mut status, &mut last_sent));
        let StatusUpdate::Audio(received) = receiver.recv().expect("audio status") else {
            panic!("expected audio status");
        };
        assert_eq!(received.volume_sync_token, Some(11));
        assert_eq!(status.volume_sync_token, None);
        assert_eq!(
            last_sent.as_ref().and_then(|sent| sent.volume_sync_token),
            None
        );
    }
}
