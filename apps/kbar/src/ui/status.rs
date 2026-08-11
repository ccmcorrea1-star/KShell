//! Right-hand status group and module-specific update methods.

use std::sync::mpsc;

use gtk::prelude::*;
use gtk4 as gtk;

use crate::services::{AudioStatus, BatteryStatus, NetworkStatus, VolumeAction};
use crate::ui::popover::PopoverCoordinator;
use crate::ui::status_icon::{status_item, IconKind, IconState, StatusIcon};
use crate::ui::volume::VolumeWidget;
use kshell_theme::tokens;

pub struct StatusWidget {
    container: gtk::Box,
    volume: VolumeWidget,
    network_icon: StatusIcon,
    network_item: gtk::Box,
    battery_icon: StatusIcon,
    battery_label: gtk::Label,
    battery_item: gtk::Box,
}

impl StatusWidget {
    pub fn new(
        coordinator: &PopoverCoordinator,
        action_sender: mpsc::Sender<VolumeAction>,
    ) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, tokens::STATUS_GAP);
        container.add_css_class("kbar-status");
        container.set_valign(gtk::Align::Center);
        container.set_baseline_position(gtk::BaselinePosition::Center);

        let (volume_item, volume) = VolumeWidget::new(coordinator, action_sender);

        let network_icon = StatusIcon::new(IconKind::Network);
        let network_item = status_item(&network_icon, None);
        network_item.set_tooltip_text(Some("Rede desconectada"));

        let battery_icon = StatusIcon::new(IconKind::Battery);
        let battery_label = gtk::Label::new(None);
        battery_label.add_css_class("kbar-status-label");
        let battery_item = status_item(&battery_icon, Some(&battery_label));
        battery_item.set_tooltip_text(Some("Bateria"));
        battery_item.set_visible(false);

        container.append(&volume_item);
        container.append(&network_item);
        container.append(&battery_item);

        Self {
            container,
            volume,
            network_icon,
            network_item,
            battery_icon,
            battery_label,
            battery_item,
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    pub fn update_audio(&self, status: &AudioStatus) {
        self.volume.update(status);
    }

    pub fn update_network(&self, status: NetworkStatus) {
        self.network_icon.set_state(IconState::Network {
            connected: status.connected,
        });
        let tooltip = if status.connected {
            "Rede conectada"
        } else {
            "Rede desconectada"
        };
        if self.network_item.tooltip_text().as_deref() != Some(tooltip) {
            self.network_item.set_tooltip_text(Some(tooltip));
        }
    }

    pub fn update_battery(&self, status: Option<BatteryStatus>) {
        if let Some(battery) = status {
            let label = format!("{}%", battery.percent);
            if self.battery_label.text().as_str() != label {
                self.battery_label.set_label(&label);
            }
            self.battery_icon.set_state(IconState::Battery {
                percent: battery.percent,
                charging: battery.charging,
            });
            if !self.battery_item.is_visible() {
                self.battery_item.set_visible(true);
            }
        } else if self.battery_item.is_visible() {
            self.battery_item.set_visible(false);
        }
    }
}
