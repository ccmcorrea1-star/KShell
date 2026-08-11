//! Small Cairo-rendered status icons shared by status widgets.

use std::cell::Cell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use kshell_theme::tokens;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IconKind {
    Volume,
    Network,
    Battery,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IconState {
    Volume { percent: Option<u8>, muted: bool },
    Network { connected: bool },
    Battery { percent: u8, charging: bool },
}

#[derive(Clone)]
pub(crate) struct StatusIcon {
    pub(crate) area: gtk::DrawingArea,
    state: Rc<Cell<IconState>>,
}

impl StatusIcon {
    pub(crate) fn new(kind: IconKind) -> Self {
        let area = gtk::DrawingArea::new();
        area.add_css_class("kbar-status-icon");
        area.set_content_width(tokens::STATUS_ICON_SIZE);
        area.set_content_height(tokens::STATUS_ICON_SIZE);
        let state = Rc::new(Cell::new(default_icon_state(kind)));
        let state_for_draw = Rc::clone(&state);
        area.set_draw_func(move |_, context, width, height| {
            draw_icon(
                context,
                kind,
                state_for_draw.get(),
                f64::from(width),
                f64::from(height),
            );
        });
        Self { area, state }
    }

    pub(crate) fn set_state(&self, state: IconState) {
        if !icon_state_changed(self.state.get(), state) {
            return;
        }
        self.state.set(state);
        self.area.queue_draw();
    }
}

pub(crate) fn status_item(icon: &StatusIcon, label: Option<&gtk::Label>) -> gtk::Box {
    let item = gtk::Box::new(gtk::Orientation::Horizontal, tokens::STATUS_LABEL_GAP);
    item.add_css_class("kbar-status-item");
    item.set_height_request(tokens::STATUS_ICON_SIZE);
    item.set_valign(gtk::Align::Center);
    item.set_baseline_position(gtk::BaselinePosition::Center);
    icon.area.set_valign(gtk::Align::Center);
    icon.area.set_vexpand(false);
    item.append(&icon.area);
    if let Some(label) = label {
        item.append(label);
        label.set_valign(gtk::Align::Center);
        label.set_margin_top(0);
        label.set_margin_bottom(0);
    }
    item
}

fn icon_state_changed(current: IconState, next: IconState) -> bool {
    current != next
}

fn default_icon_state(kind: IconKind) -> IconState {
    match kind {
        IconKind::Volume => IconState::Volume {
            percent: None,
            muted: false,
        },
        IconKind::Network => IconState::Network { connected: false },
        IconKind::Battery => IconState::Battery {
            percent: 0,
            charging: false,
        },
    }
}

fn draw_icon(
    context: &gtk::cairo::Context,
    kind: IconKind,
    state: IconState,
    width: f64,
    height: f64,
) {
    let (red, green, blue) = parse_hex_color(tokens::TEXT_SECONDARY);
    context.set_source_rgb(red, green, blue);
    context.set_line_width(tokens::ICON_STROKE_WIDTH);
    context.set_line_cap(gtk::cairo::LineCap::Round);
    context.set_line_join(gtk::cairo::LineJoin::Round);

    let scale = (width.min(height) / 16.0).max(0.1);
    context.save().ok();
    context.translate((width - 16.0 * scale) / 2.0, (height - 16.0 * scale) / 2.0);
    context.scale(scale, scale);

    match (kind, state) {
        (IconKind::Volume, IconState::Volume { percent, muted }) => {
            draw_volume(context, percent, muted)
        }
        (IconKind::Network, IconState::Network { connected }) => {
            context.translate(0.0, -2.0);
            draw_network(context, connected);
        }
        (IconKind::Battery, IconState::Battery { percent, charging }) => {
            draw_battery(context, percent, charging)
        }
        _ => {}
    }
    context.restore().ok();
}

fn draw_volume(context: &gtk::cairo::Context, percent: Option<u8>, muted: bool) {
    context.move_to(2.5, 6.25);
    context.line_to(4.75, 6.25);
    context.line_to(8.0, 3.5);
    context.line_to(8.0, 12.5);
    context.line_to(4.75, 9.75);
    context.line_to(2.5, 9.75);
    context.close_path();
    context.stroke().ok();

    if muted {
        context.move_to(10.0, 5.5);
        context.line_to(14.0, 10.5);
        context.move_to(14.0, 5.5);
        context.line_to(10.0, 10.5);
        context.stroke().ok();
        return;
    }

    let Some(percent) = percent else {
        context.move_to(10.0, 8.0);
        context.line_to(14.0, 8.0);
        context.stroke().ok();
        return;
    };

    let wave_count = match volume_level(percent) {
        VolumeLevel::Low => 1,
        VolumeLevel::Medium => 2,
        VolumeLevel::High => 3,
    };
    for radius in [3.0, 5.0, 7.0].into_iter().take(wave_count) {
        context.arc(8.0, 8.0, radius, -0.7, 0.7);
        context.stroke().ok();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VolumeLevel {
    Low,
    Medium,
    High,
}

fn volume_level(percent: u8) -> VolumeLevel {
    match percent {
        0..=33 => VolumeLevel::Low,
        34..=66 => VolumeLevel::Medium,
        _ => VolumeLevel::High,
    }
}

fn draw_network(context: &gtk::cairo::Context, connected: bool) {
    if connected {
        context.arc(8.0, 12.75, 1.0, 0.0, std::f64::consts::TAU);
        context.fill().ok();
        context.arc(8.0, 12.75, 4.0, -2.35, -0.8);
        context.stroke().ok();
        context.arc(8.0, 12.75, 6.5, -2.35, -0.8);
        context.stroke().ok();
    } else {
        context.arc(8.0, 12.75, 1.0, 0.0, std::f64::consts::TAU);
        context.fill().ok();
        context.arc(8.0, 12.75, 6.5, -2.35, -0.8);
        context.stroke().ok();
        context.move_to(3.0, 3.0);
        context.line_to(13.0, 13.0);
        context.stroke().ok();
    }
}

fn draw_battery(context: &gtk::cairo::Context, percent: u8, _charging: bool) {
    context.rectangle(2.0, 4.5, 11.0, 7.0);
    context.stroke().ok();
    context.rectangle(13.0, 7.0, 1.5, 2.0);
    context.stroke().ok();

    let level = 7.5 * f64::from(percent.min(100)) / 100.0;
    if level > 0.0 {
        context.rectangle(4.0, 6.5, level, 3.0);
        context.fill().ok();
    }
}

fn parse_hex_color(value: &str) -> (f64, f64, f64) {
    let value = value.strip_prefix('#').unwrap_or_default().as_bytes();
    (
        color_component(value.get(0..2)),
        color_component(value.get(2..4)),
        color_component(value.get(4..6)),
    )
}

fn color_component(value: Option<&[u8]>) -> f64 {
    value
        .and_then(|value| std::str::from_utf8(value).ok())
        .and_then(|value| u8::from_str_radix(value, 16).ok())
        .map(|value| f64::from(value) / 255.0)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{icon_state_changed, volume_level, IconState, VolumeLevel};

    #[test]
    fn volume_icon_uses_low_medium_and_high_thresholds() {
        assert_eq!(volume_level(0), VolumeLevel::Low);
        assert_eq!(volume_level(33), VolumeLevel::Low);
        assert_eq!(volume_level(34), VolumeLevel::Medium);
        assert_eq!(volume_level(66), VolumeLevel::Medium);
        assert_eq!(volume_level(67), VolumeLevel::High);
        assert_eq!(volume_level(100), VolumeLevel::High);
    }

    #[test]
    fn identical_icon_state_does_not_request_a_redraw() {
        let state = IconState::Network { connected: true };
        assert!(!icon_state_changed(state, state));
        assert!(icon_state_changed(
            state,
            IconState::Network { connected: false }
        ));
    }
}
