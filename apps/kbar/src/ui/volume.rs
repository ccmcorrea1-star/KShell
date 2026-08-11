//! Volume controls. The slider state machine is intentionally kept intact.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use gtk::gdk;
use gtk::prelude::*;
use gtk4 as gtk;

use crate::services::{AudioStatus, OutputDevice, VolumeAction, VolumeStatus};
use crate::ui::popover::{PopoverCoordinator, PopoverId};
use crate::ui::status_icon::{status_item, IconKind, IconState, StatusIcon};
use kshell_theme::tokens;

#[derive(Clone)]
pub struct VolumeWidget {
    volume_icon: StatusIcon,
    volume_label: gtk::Label,
    popover_icon: StatusIcon,
    popover_percent: gtk::Label,
    volume_scale: gtk::Scale,
    slider_state: Rc<Cell<SliderInteractionState>>,
    output_empty_label: gtk::Label,
    output_list: gtk::ListBox,
    rendered_output_menu: Rc<RefCell<Option<OutputMenuState>>>,
    action_sender: mpsc::Sender<VolumeAction>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SliderInteractionState {
    pub pointer_active: bool,
    pub pending_value: Option<u8>,
    pub waiting_for_sync: Option<u64>,
    pub next_sync_token: u64,
}

impl SliderInteractionState {
    pub fn value_changed(&mut self, percent: u8) {
        // A new local value supersedes any older release Sync. In particular,
        // an old response must not finish a newer keyboard or pointer action.
        self.waiting_for_sync = None;
        self.pending_value = Some(percent);
    }

    pub fn begin_pointer(&mut self, percent: u8) {
        self.waiting_for_sync = None;
        self.pointer_active = true;
        self.pending_value = Some(percent);
    }

    pub fn finish_pointer(&mut self, percent: u8) -> Option<(u64, u8)> {
        if !self.pointer_active {
            return None;
        }

        self.pointer_active = false;
        self.pending_value = Some(percent);
        let token = self.next_sync_token.wrapping_add(1);
        self.next_sync_token = token;
        self.waiting_for_sync = Some(token);
        Some((token, percent))
    }

    pub fn complete_sync(&mut self, token: u64) -> bool {
        if self.waiting_for_sync != Some(token) {
            return false;
        }

        self.waiting_for_sync = None;
        self.pending_value = None;
        true
    }

    fn preserves_local_value(&self) -> bool {
        self.pointer_active || self.waiting_for_sync.is_some()
    }
}

#[derive(Clone)]
struct ScaleInteraction {
    scale: gtk::Scale,
    state: Rc<Cell<SliderInteractionState>>,
    pending_set_value: Rc<Cell<Option<u8>>>,
    set_timer: Rc<RefCell<Option<gtk::glib::SourceId>>>,
    action_sender: mpsc::Sender<VolumeAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OutputMenuState {
    current_output_id: Option<u32>,
    outputs: Vec<OutputDevice>,
}

impl VolumeWidget {
    pub fn new(
        coordinator: &PopoverCoordinator,
        action_sender: mpsc::Sender<VolumeAction>,
    ) -> (gtk::Box, Self) {
        let volume_icon = StatusIcon::new(IconKind::Volume);
        let volume_label = gtk::Label::new(Some("—%"));
        volume_label.add_css_class("kbar-status-label");
        volume_label.set_width_chars(4);
        volume_label.set_max_width_chars(4);
        volume_label.set_xalign(1.0);
        let volume_item = status_item(&volume_icon, Some(&volume_label));
        volume_item.add_css_class("kbar-volume");
        volume_item.set_size_request(tokens::VOLUME_MODULE_WIDTH, tokens::STATUS_ICON_SIZE);
        volume_item.set_hexpand(false);
        volume_item.set_vexpand(false);
        volume_item.set_tooltip_text(Some("Volume"));

        let popover = gtk::Popover::new();
        popover.add_css_class("kbar-popover");
        popover.add_css_class("kbar-volume-popover");
        popover.set_autohide(true);
        popover.set_has_arrow(false);
        popover.set_position(gtk::PositionType::Bottom);
        popover.set_offset(
            0,
            (tokens::BAR_HEIGHT - tokens::STATUS_ICON_SIZE) / 2 + tokens::SPACE_2,
        );
        popover.set_parent(&volume_item);
        coordinator.register(PopoverId::Volume, &popover);

        let coordinator_for_close = coordinator.clone();
        popover.connect_closed(move |_| coordinator_for_close.close(PopoverId::Volume));
        let refresh_action_sender = action_sender.clone();
        popover.connect_show(move |_| {
            let _ = refresh_action_sender.send(VolumeAction::RefreshOutputs);
        });

        let content = gtk::Box::new(gtk::Orientation::Vertical, tokens::SPACE_3);
        content.add_css_class("kbar-volume-content");
        content.set_width_request(tokens::VOLUME_POPOVER_WIDTH);
        content.set_margin_top(tokens::SPACE_2);
        content.set_margin_bottom(tokens::SPACE_2);
        content.set_margin_start(tokens::SPACE_2);
        content.set_margin_end(tokens::SPACE_2);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, tokens::SPACE_2);
        header.add_css_class("kbar-volume-header");
        header.set_valign(gtk::Align::Center);

        let popover_icon = StatusIcon::new(IconKind::Volume);
        let mute_button = gtk::Button::new();
        mute_button.add_css_class("kbar-volume-mute-button");
        mute_button.set_child(Some(&popover_icon.area));
        mute_button.set_tooltip_text(Some("Alternar mudo"));
        let mute_action_sender = action_sender.clone();
        mute_button.connect_clicked(move |_| {
            let _ = mute_action_sender.send(VolumeAction::ToggleMute);
        });

        let title = gtk::Label::new(Some("Volume"));
        title.add_css_class("kbar-volume-title");
        title.set_halign(gtk::Align::Start);
        title.set_hexpand(true);

        let popover_percent = gtk::Label::new(Some("—%"));
        popover_percent.add_css_class("kbar-volume-percent");
        popover_percent.set_halign(gtk::Align::End);

        header.append(&mute_button);
        header.append(&title);
        header.append(&popover_percent);
        content.append(&header);

        let volume_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
        volume_scale.add_css_class("kbar-volume-scale");
        volume_scale.set_hexpand(true);
        volume_scale.set_draw_value(false);
        volume_scale.set_digits(0);
        volume_scale.set_increments(1.0, 5.0);
        let slider_state = Rc::new(Cell::new(SliderInteractionState::default()));
        let pending_set_value = Rc::new(Cell::new(None));
        let set_timer = Rc::new(RefCell::new(None));
        let scale_interaction = ScaleInteraction {
            scale: volume_scale.clone(),
            state: Rc::clone(&slider_state),
            pending_set_value: Rc::clone(&pending_set_value),
            set_timer: Rc::clone(&set_timer),
            action_sender: action_sender.clone(),
        };
        let interaction_for_change = scale_interaction.clone();
        volume_scale.connect_change_value(move |_, _, value| {
            let percent = value.round().clamp(0.0, 100.0) as u8;
            let mut state = interaction_for_change.state.get();
            state.value_changed(percent);
            interaction_for_change.state.set(state);
            schedule_volume_set(
                percent,
                &interaction_for_change.pending_set_value,
                &interaction_for_change.set_timer,
                &interaction_for_change.action_sender,
            );
            gtk::glib::Propagation::Proceed
        });

        let scale_pointer = gtk::GestureClick::new();
        scale_pointer.set_button(gdk::BUTTON_PRIMARY);
        scale_pointer.set_exclusive(false);
        scale_pointer.set_propagation_phase(gtk::PropagationPhase::Capture);
        let interaction_for_press = scale_interaction.clone();
        scale_pointer.connect_pressed(move |_, _, _, _| {
            interaction_for_press.begin();
        });
        let interaction_for_release = scale_interaction.clone();
        scale_pointer.connect_released(move |_, _, _, _| {
            interaction_for_release.finish();
        });
        volume_scale.add_controller(scale_pointer);

        // GtkGestureClick may stop recognizing a sequence as soon as it becomes
        // a real drag. It is therefore only used for the press/release boundary;
        // GestureDrag owns the drag lifecycle and its end signal.
        let scale_drag = gtk::GestureDrag::new();
        scale_drag.set_button(gdk::BUTTON_PRIMARY);
        scale_drag.set_exclusive(false);
        scale_drag.set_propagation_phase(gtk::PropagationPhase::Capture);
        let interaction_for_drag_begin = scale_interaction.clone();
        scale_drag.connect_drag_begin(move |_, _, _| {
            interaction_for_drag_begin.begin();
        });
        let interaction_for_drag_end = scale_interaction.clone();
        scale_drag.connect_drag_end(move |_, _, _| {
            interaction_for_drag_end.finish();
        });
        let interaction_for_drag_cancel = scale_interaction.clone();
        scale_drag.connect_cancel(move |_, _| {
            interaction_for_drag_cancel.finish();
        });
        volume_scale.add_controller(scale_drag);
        content.append(&volume_scale);

        let output_section = gtk::Box::new(gtk::Orientation::Vertical, tokens::SPACE_2);
        output_section.add_css_class("kbar-volume-output-section");
        let output_heading = gtk::Label::new(Some("Saída"));
        output_heading.add_css_class("kbar-volume-output-heading");
        output_heading.set_halign(gtk::Align::Start);
        let output_empty_label = gtk::Label::new(Some("Nenhuma saída disponível"));
        output_empty_label.add_css_class("kbar-volume-output-empty");
        output_empty_label.set_halign(gtk::Align::Start);
        output_empty_label.set_single_line_mode(true);
        output_empty_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let output_list = gtk::ListBox::new();
        output_list.add_css_class("kbar-volume-output-list");
        output_list.set_selection_mode(gtk::SelectionMode::None);
        output_list.set_show_separators(false);
        output_list.set_visible(false);
        output_section.append(&output_heading);
        output_section.append(&output_empty_label);
        output_section.append(&output_list);
        content.append(&output_section);
        popover.set_child(Some(&content));

        let left_click = gtk::GestureClick::new();
        left_click.set_button(gdk::BUTTON_PRIMARY);
        left_click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let popover_for_click = popover.clone();
        let coordinator_for_click = coordinator.clone();
        left_click.connect_pressed(move |_, press_count, _, _| {
            if press_count == 1 {
                coordinator_for_click.open(PopoverId::Volume);
                popover_for_click.popup();
            }
        });
        volume_item.add_controller(left_click);

        let middle_click = gtk::GestureClick::new();
        middle_click.set_button(gdk::BUTTON_MIDDLE);
        middle_click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let middle_action_sender = action_sender.clone();
        middle_click.connect_pressed(move |_, _, _, _| {
            let _ = middle_action_sender.send(VolumeAction::ToggleMute);
        });
        volume_item.add_controller(middle_click);

        let scroll = gtk::EventControllerScroll::new(
            gtk::EventControllerScrollFlags::VERTICAL | gtk::EventControllerScrollFlags::DISCRETE,
        );
        scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
        let scroll_action_sender = action_sender.clone();
        scroll.connect_scroll(move |_, _, delta_y| {
            if delta_y == 0.0 {
                return gtk::glib::Propagation::Proceed;
            }
            let step = if delta_y < 0.0 { 5 } else { -5 };
            let _ = scroll_action_sender.send(VolumeAction::Adjust(step));
            gtk::glib::Propagation::Stop
        });
        volume_item.add_controller(scroll);

        let widget = Self {
            volume_icon,
            volume_label,
            popover_icon,
            popover_percent,
            volume_scale,
            slider_state: scale_interaction.state,
            output_empty_label,
            output_list,
            rendered_output_menu: Rc::new(RefCell::new(None)),
            action_sender,
        };
        (volume_item, widget)
    }

    pub fn update(&self, status: &AudioStatus) {
        self.apply_status(&status.volume, status.volume_sync_token);
    }

    fn apply_status(&self, status: &VolumeStatus, sync_token: Option<u64>) {
        let mut slider_state = self.slider_state.get();
        if let Some(sync_token) = sync_token {
            slider_state.complete_sync(sync_token);
        }

        let preserve_local_value = slider_state.preserves_local_value();
        let displayed_percent = volume_percent_for_display(
            slider_state.pending_value,
            slider_state.pointer_active,
            slider_state.waiting_for_sync.is_some(),
            status.percent,
        );
        let volume_label = displayed_percent
            .map(|percent| format!("{percent}%"))
            .unwrap_or_else(|| "—%".to_owned());
        set_label_if_changed(&self.volume_label, &volume_label);
        set_label_if_changed(&self.popover_percent, &volume_label);

        let volume_state = IconState::Volume {
            percent: displayed_percent,
            muted: status.muted,
        };
        self.volume_icon.set_state(volume_state);
        self.popover_icon.set_state(volume_state);

        if let Some(percent) = displayed_percent {
            if !self.volume_scale.is_sensitive() {
                self.volume_scale.set_sensitive(true);
            }
            if !preserve_local_value {
                let value = f64::from(percent);
                if (self.volume_scale.value() - value).abs() > f64::EPSILON {
                    self.volume_scale.set_value(value);
                }
            }
        } else if self.volume_scale.is_sensitive() {
            self.volume_scale.set_sensitive(false);
        }

        if !preserve_local_value {
            slider_state.pending_value = None;
        }
        self.slider_state.set(slider_state);

        let current_output_id = status.current_output.as_ref().map(|output| output.id);
        let has_outputs = !status.outputs.is_empty();
        if self.output_empty_label.is_visible() == has_outputs {
            self.output_empty_label.set_visible(!has_outputs);
        }
        let show_output_list = should_show_output_list(status.outputs.len());
        if self.output_list.is_visible() != show_output_list {
            self.output_list.set_visible(show_output_list);
        }

        let output_menu_state = OutputMenuState {
            current_output_id,
            outputs: status.outputs.clone(),
        };
        let menu_changed = self.rendered_output_menu.borrow().as_ref() != Some(&output_menu_state);
        if !menu_changed {
            return;
        }

        self.output_list.remove_all();
        for output in &output_menu_state.outputs {
            let button = gtk::Button::new();
            button.add_css_class("kbar-volume-output");
            button.set_hexpand(true);
            button.set_halign(gtk::Align::Fill);
            button.set_tooltip_text(Some(&format!("Usar {}", output.name)));
            if current_output_id == Some(output.id) {
                button.add_css_class("is-active");
            }

            let label = gtk::Label::new(Some(&output.name));
            label.set_halign(gtk::Align::Start);
            label.set_hexpand(true);
            label.set_single_line_mode(true);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            button.set_child(Some(&label));

            let action_sender = self.action_sender.clone();
            let output_id = output.id;
            button.connect_clicked(move |_| {
                let _ = action_sender.send(VolumeAction::SetDefault(output_id));
            });
            self.output_list.append(&button);
        }
        *self.rendered_output_menu.borrow_mut() = Some(output_menu_state);
    }
}

fn scale_percent(scale: &gtk::Scale) -> u8 {
    scale.value().round().clamp(0.0, 100.0) as u8
}

impl ScaleInteraction {
    fn begin(&self) {
        let mut state = self.state.get();
        state.begin_pointer(scale_percent(&self.scale));
        self.state.set(state);
    }

    fn finish(&self) {
        let mut state = self.state.get();
        let Some((token, percent)) = state.finish_pointer(scale_percent(&self.scale)) else {
            return;
        };
        self.state.set(state);

        flush_volume_set(
            &self.pending_set_value,
            &self.set_timer,
            percent,
            &self.action_sender,
        );
        let _ = self.action_sender.send(VolumeAction::Sync {
            token,
            requested_percent: percent,
        });
    }
}

const VOLUME_SET_THROTTLE: Duration = Duration::from_millis(40);

fn schedule_volume_set(
    percent: u8,
    pending_value: &Rc<Cell<Option<u8>>>,
    timer: &Rc<RefCell<Option<gtk::glib::SourceId>>>,
    action_sender: &mpsc::Sender<VolumeAction>,
) {
    pending_value.set(Some(percent));
    if timer.borrow().is_some() {
        return;
    }

    let pending_value_for_timer = Rc::clone(pending_value);
    let timer_for_callback = Rc::clone(timer);
    let action_sender_for_timer = action_sender.clone();
    let source_id = gtk::glib::timeout_add_local(VOLUME_SET_THROTTLE, move || {
        timer_for_callback.borrow_mut().take();
        if let Some(percent) = pending_value_for_timer.take() {
            let _ = action_sender_for_timer.send(VolumeAction::Set(percent));
        }
        gtk::glib::ControlFlow::Break
    });
    *timer.borrow_mut() = Some(source_id);
}

fn flush_volume_set(
    pending_value: &Cell<Option<u8>>,
    timer: &RefCell<Option<gtk::glib::SourceId>>,
    percent: u8,
    action_sender: &mpsc::Sender<VolumeAction>,
) {
    if let Some(source_id) = timer.borrow_mut().take() {
        source_id.remove();
    }
    pending_value.set(None);
    let _ = action_sender.send(VolumeAction::Set(percent));
}

fn set_label_if_changed(label: &gtk::Label, text: &str) {
    if label.text().as_str() != text {
        label.set_label(text);
    }
}

fn volume_percent_for_display(
    local_value: Option<u8>,
    pointer_active: bool,
    sync_pending: bool,
    external_value: Option<u8>,
) -> Option<u8> {
    if pointer_active || sync_pending {
        local_value.or(external_value)
    } else {
        external_value
    }
}

fn should_show_output_list(output_count: usize) -> bool {
    output_count > 0
}

#[cfg(test)]
mod tests {
    use super::{should_show_output_list, volume_percent_for_display, SliderInteractionState};

    #[test]
    fn local_slider_value_wins_until_external_sync_finishes() {
        assert_eq!(
            volume_percent_for_display(Some(83), true, false, Some(61)),
            Some(83)
        );
        assert_eq!(
            volume_percent_for_display(Some(83), false, true, Some(61)),
            Some(83)
        );
        assert_eq!(
            volume_percent_for_display(Some(83), false, false, Some(61)),
            Some(61)
        );
    }

    #[test]
    fn change_value_does_not_start_pointer_interaction() {
        let mut state = SliderInteractionState::default();
        state.value_changed(72);
        assert!(!state.pointer_active);
        assert_eq!(state.pending_value, Some(72));
        assert_eq!(state.waiting_for_sync, None);
    }

    #[test]
    fn pointer_lifecycle_keeps_external_updates_off_the_thumb() {
        let mut state = SliderInteractionState::default();
        state.begin_pointer(40);
        state.value_changed(83);
        assert_eq!(
            volume_percent_for_display(
                state.pending_value,
                state.pointer_active,
                state.waiting_for_sync.is_some(),
                Some(61),
            ),
            Some(83)
        );

        let (token, percent) = state.finish_pointer(83).expect("pointer was active");
        assert_eq!(percent, 83);
        assert!(!state.pointer_active);
        assert_eq!(state.waiting_for_sync, Some(token));
    }

    #[test]
    fn old_sync_does_not_finish_a_newer_interaction() {
        let mut state = SliderInteractionState::default();
        state.begin_pointer(40);
        let (old_token, _) = state.finish_pointer(50).expect("first pointer interaction");
        state.begin_pointer(50);
        let (new_token, _) = state
            .finish_pointer(70)
            .expect("second pointer interaction");
        assert!(!state.complete_sync(old_token));
        assert_eq!(state.waiting_for_sync, Some(new_token));
    }

    #[test]
    fn confirmed_sync_returns_backend_to_being_the_source_of_truth() {
        let mut state = SliderInteractionState::default();
        state.begin_pointer(40);
        let (token, _) = state.finish_pointer(83).expect("pointer interaction");
        assert!(state.complete_sync(token));
        assert_eq!(
            volume_percent_for_display(
                state.pending_value,
                state.pointer_active,
                state.waiting_for_sync.is_some(),
                Some(61),
            ),
            Some(61)
        );
    }

    #[test]
    fn shows_output_choices_when_outputs_exist() {
        assert!(!should_show_output_list(0));
        assert!(should_show_output_list(1));
        assert!(should_show_output_list(2));
    }
}
