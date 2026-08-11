//! Background sources used by Kbar.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use futures_channel::mpsc::UnboundedSender;

pub mod audio;
pub mod battery;
mod command;
pub mod network;

pub use audio::{AudioStatus, OutputDevice, VolumeAction, VolumeStatus};
pub use battery::BatteryStatus;
pub use network::NetworkStatus;

const STATUS_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SystemStatus {
    pub volume: VolumeStatus,
    pub volume_sync_token: Option<u64>,
    pub network_connected: bool,
    pub battery: Option<BatteryStatus>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SlowSystemStatus {
    pub network: NetworkStatus,
    pub battery: Option<BatteryStatus>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatusUpdate {
    Audio(AudioStatus),
    SlowSystem(SlowSystemStatus),
}

pub fn spawn_status_worker(sender: UnboundedSender<StatusUpdate>) -> Sender<VolumeAction> {
    let (action_sender, action_receiver) = mpsc::channel();
    let (update_sender, update_receiver) = mpsc::channel();

    audio::spawn_audio_worker(audio::WpctlBackend, action_receiver, update_sender.clone());
    spawn_system_worker(update_sender);
    thread::spawn(move || aggregate_status_updates(update_receiver, sender));

    action_sender
}

pub fn spawn_system_worker(update_sender: Sender<StatusUpdate>) {
    thread::spawn(move || {
        let mut status = SlowSystemStatus {
            network: network::read(),
            battery: battery::read(),
        };
        let mut last_sent = None;

        if !send_slow_system_status(&update_sender, &status, &mut last_sent) {
            return;
        }

        loop {
            thread::sleep(STATUS_REFRESH_INTERVAL);
            let next_status = SlowSystemStatus {
                network: network::read(),
                battery: battery::read(),
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

/// Coalesces status snapshots without turning them back into one UI snapshot.
///
/// `SystemStatus` remains here as a small internal aggregate so the
/// deduplication semantics from the audio refactor stay testable. The UI only
/// receives the latest changed `StatusUpdate` variant.
pub fn aggregate_status_updates(
    receiver: Receiver<StatusUpdate>,
    sender: UnboundedSender<StatusUpdate>,
) {
    let mut status = SystemStatus::default();
    let mut has_audio = false;
    let mut has_slow_system = false;
    let mut last_audio = None;
    let mut last_slow_system = None;

    while let Ok(first_update) = receiver.recv() {
        let mut latest_audio = None;
        let mut latest_slow_system = None;
        let mut audio_changed = false;
        let mut slow_system_changed = false;

        for update in std::iter::once(first_update).chain(receiver.try_iter()) {
            let snapshot = update.clone();
            let changed =
                merge_status_update(&mut status, update, &mut has_audio, &mut has_slow_system);
            if !changed {
                continue;
            }
            match snapshot {
                StatusUpdate::Audio(audio) => {
                    latest_audio = Some(audio);
                    audio_changed = true;
                }
                StatusUpdate::SlowSystem(slow_system) => {
                    latest_slow_system = Some(slow_system);
                    slow_system_changed = true;
                }
            }
        }

        if audio_changed {
            if let Some(audio) = latest_audio {
                if last_audio.as_ref() != Some(&audio) {
                    if sender
                        .unbounded_send(StatusUpdate::Audio(audio.clone()))
                        .is_err()
                    {
                        return;
                    }
                    last_audio = Some(audio);
                }
            }
        }
        if slow_system_changed {
            if let Some(slow_system) = latest_slow_system {
                if last_slow_system.as_ref() != Some(&slow_system) {
                    if sender
                        .unbounded_send(StatusUpdate::SlowSystem(slow_system.clone()))
                        .is_err()
                    {
                        return;
                    }
                    last_slow_system = Some(slow_system);
                }
            }
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
                || status.network_connected != system.network.connected
                || status.battery != system.battery;
            status.network_connected = system.network.connected;
            status.battery = system.battery;
            *has_slow_system = true;
            changed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        aggregate_status_updates, merge_status_update, AudioStatus, NetworkStatus,
        SlowSystemStatus, StatusUpdate, SystemStatus,
    };
    use std::sync::mpsc;

    #[test]
    fn identical_internal_status_updates_are_deduplicated() {
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
        let slow = SlowSystemStatus {
            network: NetworkStatus { connected: true },
            battery: None,
        };
        assert!(merge_status_update(
            &mut status,
            StatusUpdate::SlowSystem(slow.clone()),
            &mut has_audio,
            &mut has_slow_system,
        ));
        assert!(!merge_status_update(
            &mut status,
            StatusUpdate::SlowSystem(slow),
            &mut has_audio,
            &mut has_slow_system,
        ));
    }

    #[test]
    fn aggregate_keeps_audio_and_slow_system_events_independent() {
        let (sender, receiver) = mpsc::channel();
        let (event_sender, mut event_receiver) = futures_channel::mpsc::unbounded();
        sender
            .send(StatusUpdate::Audio(AudioStatus::default()))
            .unwrap();
        sender
            .send(StatusUpdate::SlowSystem(SlowSystemStatus::default()))
            .unwrap();
        drop(sender);

        aggregate_status_updates(receiver, event_sender);
        let first = event_receiver.try_recv().unwrap();
        let second = event_receiver.try_recv().unwrap();
        assert!(matches!(first, StatusUpdate::Audio(_)));
        assert!(matches!(second, StatusUpdate::SlowSystem(_)));
    }
}
