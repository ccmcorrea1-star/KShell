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

        if update_sender
            .send(StatusUpdate::SlowSystem(status.clone()))
            .is_err()
        {
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
            if update_sender
                .send(StatusUpdate::SlowSystem(status.clone()))
                .is_err()
            {
                break;
            }
        }
    });
}

/// Coalesces status snapshots while keeping audio and slow system events
/// independent for the UI.
pub fn aggregate_status_updates(
    receiver: Receiver<StatusUpdate>,
    sender: UnboundedSender<StatusUpdate>,
) {
    let mut last_audio = None;
    let mut last_slow_system = None;

    while let Ok(first_update) = receiver.recv() {
        let mut latest_audio = None;
        let mut latest_slow_system = None;

        for update in std::iter::once(first_update).chain(receiver.try_iter()) {
            match update {
                StatusUpdate::Audio(audio) => {
                    latest_audio = Some(audio);
                }
                StatusUpdate::SlowSystem(slow_system) => {
                    latest_slow_system = Some(slow_system);
                }
            }
        }

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

#[cfg(test)]
mod tests {
    use super::{aggregate_status_updates, AudioStatus, SlowSystemStatus, StatusUpdate};
    use std::sync::mpsc;

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

    #[test]
    fn aggregate_deduplicates_repeated_snapshots() {
        let (sender, receiver) = mpsc::channel();
        let (event_sender, mut event_receiver) = futures_channel::mpsc::unbounded();
        sender
            .send(StatusUpdate::Audio(AudioStatus::default()))
            .unwrap();
        sender
            .send(StatusUpdate::Audio(AudioStatus::default()))
            .unwrap();
        sender
            .send(StatusUpdate::SlowSystem(SlowSystemStatus::default()))
            .unwrap();
        sender
            .send(StatusUpdate::SlowSystem(SlowSystemStatus::default()))
            .unwrap();
        drop(sender);

        aggregate_status_updates(receiver, event_sender);
        assert!(matches!(
            event_receiver.try_recv(),
            Ok(StatusUpdate::Audio(_))
        ));
        assert!(matches!(
            event_receiver.try_recv(),
            Ok(StatusUpdate::SlowSystem(_))
        ));
        assert!(event_receiver.try_recv().is_err());
    }
}
