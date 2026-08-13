//! Volume controls. The slider state machine is intentionally kept intact.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use gtk::gdk;
use gtk::gdk::prelude::*;
use gtk::prelude::*;
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

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
    output_rows: Rc<RefCell<Vec<(u32, gtk::Button)>>>,
    surface: Rc<VolumeSurface>,
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
        self.pointer_active || self.waiting_for_sync.is_some() || self.pending_value.is_some()
    }
}

#[derive(Clone)]
struct ScaleInteraction {
    scale: gtk::Scale,
    state: Rc<Cell<SliderInteractionState>>,
    pending_set_value: Rc<Cell<Option<u8>>>,
    set_timer: Rc<RefCell<Option<gtk::glib::SourceId>>>,
    last_sent_value: Rc<Cell<Option<u8>>>,
    action_sender: mpsc::Sender<VolumeAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OutputMenuState {
    current_output_id: Option<u32>,
    outputs: Vec<OutputDevice>,
}

struct VolumeSurface {
    application: gtk::Application,
    main_window: gtk::ApplicationWindow,
    anchor: gtk::Box,
    monitor: Option<gdk::Monitor>,
    coordinator: PopoverCoordinator,
    action_sender: mpsc::Sender<VolumeAction>,
    panel: gtk::Box,
    focus_target: gtk::Button,
    window: RefCell<Option<gtk::ApplicationWindow>>,
    click_catcher: RefCell<Option<gtk::ApplicationWindow>>,
    effective_monitor: RefCell<Option<gdk::Monitor>>,
    logical_open: Cell<bool>,
    last_width: Cell<i32>,
    last_height: Cell<i32>,
}

struct VolumeSurfaceConfig {
    application: gtk::Application,
    main_window: gtk::ApplicationWindow,
    anchor: gtk::Box,
    monitor: Option<gdk::Monitor>,
    coordinator: PopoverCoordinator,
    action_sender: mpsc::Sender<VolumeAction>,
}

impl VolumeSurface {
    fn new(config: VolumeSurfaceConfig, content: &gtk::Box, focus_target: &gtk::Button) -> Self {
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
        panel.add_css_class("kbar-volume-popover");
        panel.append(content);

        Self {
            application: config.application,
            main_window: config.main_window,
            anchor: config.anchor,
            monitor: config.monitor,
            coordinator: config.coordinator,
            action_sender: config.action_sender,
            panel,
            focus_target: focus_target.clone(),
            window: RefCell::new(None),
            click_catcher: RefCell::new(None),
            effective_monitor: RefCell::new(None),
            logical_open: Cell::new(false),
            last_width: Cell::new(volume_surface_width_estimate()),
            last_height: Cell::new(0),
        }
    }

    fn show(self: &Rc<Self>) {
        self.logical_open.set(true);
        let window = self.ensure_window();
        let click_catcher = self.ensure_click_catcher();

        self.update_position(&window);
        let _ = self.action_sender.send(VolumeAction::RefreshOutputs);

        // The catcher is mapped first so it receives outside clicks while the
        // actual panel is mapped afterwards and remains above it on Top.
        click_catcher.set_visible(true);
        click_catcher.present();
        window.set_keyboard_mode(KeyboardMode::OnDemand);
        window.set_visible(true);
        self.update_position(&window);
        gtk::prelude::GtkWindowExt::set_focus(&window, Some(&self.focus_target));
        self.focus_target.grab_focus();
        window.present();

        let weak_self = Rc::downgrade(self);
        gtk::glib::idle_add_local_once(move || {
            let Some(surface) = weak_self.upgrade() else {
                return;
            };
            if !surface.logical_open.get() {
                return;
            }
            let window = surface.window.borrow().as_ref().cloned();
            if let Some(window) = window {
                surface.update_position(&window);
                if let Some(catcher) = surface.click_catcher.borrow().as_ref() {
                    catcher.present();
                }
                gtk::prelude::GtkWindowExt::set_focus(&window, Some(&surface.focus_target));
                surface.focus_target.grab_focus();
                window.present();
            }
        });
    }

    fn reposition_if_open(&self) {
        if !self.logical_open.get() {
            return;
        }
        let window = self.window.borrow().as_ref().cloned();
        if let Some(window) = window {
            self.update_position(&window);
        }
    }

    fn close(&self) {
        self.hide();
        self.coordinator.close(PopoverId::Volume);
    }

    fn hide(&self) {
        self.logical_open.set(false);
        if let Some(click_catcher) = self.click_catcher.borrow().as_ref() {
            click_catcher.set_visible(false);
        }
        let window = self.window.borrow().as_ref().cloned();
        if let Some(window) = window {
            window.set_keyboard_mode(KeyboardMode::None);
            gtk::prelude::GtkWindowExt::set_focus(&window, None::<&gtk::Widget>);
            window.set_visible(false);
        }
    }

    fn ensure_window(self: &Rc<Self>) -> gtk::ApplicationWindow {
        if let Some(window) = self.window.borrow().as_ref() {
            return window.clone();
        }

        let window = gtk::ApplicationWindow::builder()
            .application(&self.application)
            .decorated(false)
            .resizable(false)
            .build();
        window.add_css_class("kbar-window");
        window.add_css_class("kbar-volume-surface");
        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_exclusive_zone(0);
        window.set_namespace(Some(kshell_niri::VOLUME_NAMESPACE));
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_monitor(self.monitor.as_ref());
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Bottom, false);
        window.set_anchor(Edge::Left, false);
        window.set_child(Some(&self.panel));

        let weak_self = Rc::downgrade(self);
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key != gdk::Key::Escape {
                return gtk::glib::Propagation::Proceed;
            }
            if let Some(surface) = weak_self.upgrade() {
                surface.close();
            }
            gtk::glib::Propagation::Stop
        });
        window.add_controller(key_controller);

        let weak_self = Rc::downgrade(self);
        window.connect_close_request(move |_| {
            if let Some(surface) = weak_self.upgrade() {
                surface.close();
            }
            gtk::glib::Propagation::Stop
        });

        *self.window.borrow_mut() = Some(window.clone());
        window
    }

    fn ensure_click_catcher(self: &Rc<Self>) -> gtk::ApplicationWindow {
        if let Some(catcher) = self.click_catcher.borrow().as_ref() {
            return catcher.clone();
        }

        let catcher = gtk::ApplicationWindow::builder()
            .application(&self.application)
            .decorated(false)
            .resizable(false)
            .build();
        catcher.add_css_class("kbar-window");
        catcher.add_css_class("kbar-volume-click-catcher");
        catcher.init_layer_shell();
        catcher.set_layer(Layer::Top);
        // A negative exclusive zone makes the transparent catcher cover the
        // whole output, including areas reserved by the bar. The top margin
        // below then leaves the bar itself interactive.
        catcher.set_exclusive_zone(-1);
        catcher.set_namespace(Some(kshell_niri::VOLUME_CLICK_CATCHER_NAMESPACE));
        catcher.set_keyboard_mode(KeyboardMode::None);
        catcher.set_monitor(self.monitor.as_ref());
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            catcher.set_anchor(edge, true);
        }
        let catcher_top = tokens::BAR_HEIGHT + tokens::BAR_MARGIN;
        catcher.set_margin(Edge::Top, catcher_top);
        if let Some(monitor) = self
            .monitor
            .clone()
            .or_else(|| actual_monitor(&self.main_window))
        {
            let geometry = monitor.geometry();
            catcher.set_default_size(
                geometry.width(),
                geometry.height().saturating_sub(catcher_top),
            );
        }

        let overlay = gtk::Box::new(gtk::Orientation::Vertical, 0);
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        overlay.set_can_target(true);
        catcher.set_child(Some(&overlay));

        let gesture = gtk::GestureClick::new();
        gesture.set_button(0);
        gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak_self = Rc::downgrade(self);
        gesture.connect_released(move |_, _, _, _| {
            if let Some(surface) = weak_self.upgrade() {
                surface.close();
            }
        });
        overlay.add_controller(gesture);

        *self.click_catcher.borrow_mut() = Some(catcher.clone());
        catcher
    }

    fn update_position(&self, window: &gtk::ApplicationWindow) {
        let Some(monitor) = self
            .monitor
            .clone()
            .or_else(|| self.effective_monitor.borrow().clone())
            .or_else(|| actual_monitor(&self.main_window))
        else {
            return;
        };

        let geometry = monitor.geometry();
        let monitor_changed = self
            .effective_monitor
            .borrow()
            .as_ref()
            .is_none_or(|current| current.as_ptr() != monitor.as_ptr());
        if monitor_changed {
            self.effective_monitor.replace(Some(monitor.clone()));
            window.set_monitor(Some(&monitor));
            if let Some(catcher) = self.click_catcher.borrow().as_ref() {
                catcher.set_monitor(Some(&monitor));
                catcher.set_default_size(
                    geometry.width(),
                    geometry
                        .height()
                        .saturating_sub(tokens::BAR_HEIGHT + tokens::BAR_MARGIN),
                );
            }
        }

        let (_, measured_width, _, _) = self.panel.measure(gtk::Orientation::Horizontal, -1);
        let (_, measured_height, _, _) = self.panel.measure(
            gtk::Orientation::Vertical,
            measured_width.max(self.last_width.get()),
        );
        let width = if measured_width > 0 {
            measured_width
        } else if window.width() > 0 {
            window.width()
        } else {
            self.last_width.get()
        };
        self.last_width.set(width);
        let height = if measured_height > 0 {
            measured_height
        } else if window.height() > 0 {
            window.height()
        } else {
            self.last_height.get()
        };
        self.last_height.set(height);

        let anchor_x = self.anchor_x().unwrap_or_else(|| {
            geometry.width() - tokens::BAR_MARGIN - tokens::VOLUME_MODULE_WIDTH / 2
        });
        let top = volume_popup_top_margin_for_height(height, geometry.height(), tokens::SPACE_2);
        let right = volume_popup_right_margin(anchor_x, geometry.width(), width, tokens::SPACE_2);
        window.set_margin(Edge::Top, top);
        window.set_margin(Edge::Right, right);
        window.set_margin(Edge::Left, 0);
        window.set_margin(Edge::Bottom, 0);
    }

    fn anchor_x(&self) -> Option<i32> {
        let point = gtk::graphene::Point::new(
            self.anchor.width() as f32 / 2.0,
            self.anchor.height() as f32 / 2.0,
        );
        self.anchor
            .compute_point(&self.main_window, &point)
            .map(|point| point.x().round() as i32 + tokens::BAR_MARGIN)
    }
}

impl VolumeWidget {
    pub fn new(
        application: &gtk::Application,
        main_window: &gtk::ApplicationWindow,
        monitor: Option<gdk::Monitor>,
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
        let last_sent_value = Rc::new(Cell::new(None));
        let scale_interaction = ScaleInteraction {
            scale: volume_scale.clone(),
            state: Rc::clone(&slider_state),
            pending_set_value: Rc::clone(&pending_set_value),
            set_timer: Rc::clone(&set_timer),
            last_sent_value: Rc::clone(&last_sent_value),
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
                &interaction_for_change.last_sent_value,
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

        let surface = Rc::new(VolumeSurface::new(
            VolumeSurfaceConfig {
                application: application.clone(),
                main_window: main_window.clone(),
                anchor: volume_item.clone(),
                monitor,
                coordinator: coordinator.clone(),
                action_sender: action_sender.clone(),
            },
            &content,
            &mute_button,
        ));
        let surface_for_coordinator = Rc::downgrade(&surface);
        coordinator.register(PopoverId::Volume, move || {
            if let Some(surface) = surface_for_coordinator.upgrade() {
                surface.hide();
            }
        });

        let left_click = gtk::GestureClick::new();
        left_click.set_button(gdk::BUTTON_PRIMARY);
        left_click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let coordinator_for_click = coordinator.clone();
        let surface_for_click = Rc::clone(&surface);
        left_click.connect_pressed(move |_, press_count, _, _| {
            if press_count == 1 {
                if coordinator_for_click.is_active(PopoverId::Volume) {
                    surface_for_click.close();
                } else {
                    coordinator_for_click.open(PopoverId::Volume);
                    surface_for_click.show();
                }
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
            output_rows: Rc::new(RefCell::new(Vec::new())),
            surface,
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

        if !slider_state.pointer_active && sync_token.is_none() && status.percent.is_none() {
            slider_state.pending_value = None;
            slider_state.waiting_for_sync = None;
        }

        if !slider_state.pointer_active
            && slider_state.waiting_for_sync.is_none()
            && slider_state.pending_value == status.percent
        {
            slider_state.pending_value = None;
        }
        let preserve_local_value = slider_state.preserves_local_value();
        let displayed_percent = displayed_percent_for_slider(slider_state, status.percent);
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
        let previous_menu = self.rendered_output_menu.borrow().clone();
        let structure_changed = previous_menu
            .as_ref()
            .map(|previous| output_structure_changed(&previous.outputs, &output_menu_state.outputs))
            .unwrap_or(true);
        if !structure_changed
            && previous_menu
                .as_ref()
                .is_some_and(|previous| previous.current_output_id == current_output_id)
        {
            self.surface.reposition_if_open();
            return;
        }

        if structure_changed {
            self.output_list.remove_all();
            let mut output_rows = self.output_rows.borrow_mut();
            output_rows.clear();
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
                output_rows.push((output.id, button));
            }
        } else {
            for (output_id, button) in self.output_rows.borrow().iter() {
                if current_output_id == Some(*output_id) {
                    button.add_css_class("is-active");
                } else {
                    button.remove_css_class("is-active");
                }
            }
        }
        *self.rendered_output_menu.borrow_mut() = Some(output_menu_state);
        self.surface.reposition_if_open();
    }
}

fn scale_percent(scale: &gtk::Scale) -> u8 {
    scale.value().round().clamp(0.0, 100.0) as u8
}

impl ScaleInteraction {
    fn begin(&self) {
        self.last_sent_value.set(None);
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
            &self.last_sent_value,
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
    last_sent_value: &Rc<Cell<Option<u8>>>,
    action_sender: &mpsc::Sender<VolumeAction>,
) {
    pending_value.set(Some(percent));
    if timer.borrow().is_some() {
        return;
    }

    let pending_value_for_timer = Rc::clone(pending_value);
    let timer_for_callback = Rc::clone(timer);
    let last_sent_value_for_timer = Rc::clone(last_sent_value);
    let action_sender_for_timer = action_sender.clone();
    let source_id = gtk::glib::timeout_add_local(VOLUME_SET_THROTTLE, move || {
        timer_for_callback.borrow_mut().take();
        if let Some(percent) = pending_value_for_timer.take() {
            let _ = action_sender_for_timer.send(VolumeAction::Set(percent));
            last_sent_value_for_timer.set(Some(percent));
        }
        gtk::glib::ControlFlow::Break
    });
    *timer.borrow_mut() = Some(source_id);
}

fn flush_volume_set(
    pending_value: &Cell<Option<u8>>,
    timer: &RefCell<Option<gtk::glib::SourceId>>,
    last_sent_value: &Cell<Option<u8>>,
    percent: u8,
    action_sender: &mpsc::Sender<VolumeAction>,
) {
    if let Some(source_id) = timer.borrow_mut().take() {
        source_id.remove();
    }
    pending_value.take();
    if should_send_final_set(last_sent_value.get(), percent) {
        let _ = action_sender.send(VolumeAction::Set(percent));
        last_sent_value.set(Some(percent));
    }
}

fn should_send_final_set(last_sent_value: Option<u8>, percent: u8) -> bool {
    last_sent_value != Some(percent)
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

fn displayed_percent_for_slider(
    state: SliderInteractionState,
    external_value: Option<u8>,
) -> Option<u8> {
    if state.preserves_local_value() {
        state.pending_value.or(external_value)
    } else {
        volume_percent_for_display(
            state.pending_value,
            state.pointer_active,
            state.waiting_for_sync.is_some(),
            external_value,
        )
    }
}

fn should_show_output_list(output_count: usize) -> bool {
    output_count > 0
}

fn output_structure_changed(previous: &[OutputDevice], next: &[OutputDevice]) -> bool {
    previous.len() != next.len()
        || previous
            .iter()
            .zip(next)
            .any(|(previous, next)| previous.id != next.id || previous.name != next.name)
}

fn volume_surface_width_estimate() -> i32 {
    tokens::VOLUME_POPOVER_WIDTH + 2 * (tokens::SPACE_2 + tokens::BORDER_WIDTH) + tokens::SPACE_2
}

fn volume_popup_top_margin() -> i32 {
    // The Top layer is laid out after the bar's reserved area. Only the
    // visual gap from the old anchored popover belongs in this margin; adding
    // the bar height here positions the surface twice below the bar.
    tokens::SPACE_2
}

fn volume_popup_top_margin_for_height(
    popup_height: i32,
    monitor_height: i32,
    edge_margin: i32,
) -> i32 {
    let preferred = volume_popup_top_margin();
    let minimum = edge_margin.max(0);
    let maximum = monitor_height.saturating_sub(popup_height + minimum);
    if maximum < minimum {
        return minimum;
    }
    preferred.clamp(minimum, maximum)
}

fn volume_popup_right_margin(
    anchor_x: i32,
    monitor_width: i32,
    popup_width: i32,
    edge_margin: i32,
) -> i32 {
    let minimum = edge_margin.max(0);
    let maximum = monitor_width.saturating_sub(popup_width + minimum);
    if maximum < minimum {
        return minimum;
    }
    (monitor_width - anchor_x - popup_width / 2).clamp(minimum, maximum)
}

fn actual_monitor(window: &gtk::ApplicationWindow) -> Option<gdk::Monitor> {
    let surface = window.surface()?;
    gdk::Display::default()?.monitor_at_surface(&surface)
}

#[cfg(test)]
mod tests {
    use super::{
        displayed_percent_for_slider, output_structure_changed, should_send_final_set,
        should_show_output_list, volume_percent_for_display, volume_popup_right_margin,
        volume_popup_top_margin, volume_popup_top_margin_for_height, SliderInteractionState,
    };
    use crate::services::OutputDevice;

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
    fn a_new_local_intention_invalidates_old_sync_and_old_snapshot() {
        let mut state = SliderInteractionState::default();
        state.begin_pointer(40);
        let (old_token, _) = state.finish_pointer(50).expect("first pointer interaction");

        state.value_changed(72);

        assert_eq!(state.waiting_for_sync, None);
        assert_eq!(state.pending_value, Some(72));
        assert!(!state.complete_sync(old_token));
        assert_eq!(displayed_percent_for_slider(state, Some(41)), Some(72));
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

    #[test]
    fn final_set_is_not_repeated_after_the_throttled_value_was_sent() {
        assert!(!should_send_final_set(Some(72), 72));
        assert!(should_send_final_set(Some(71), 72));
        assert!(should_send_final_set(None, 72));
    }

    #[test]
    fn output_structure_ignores_only_the_default_marker() {
        let previous = vec![
            OutputDevice {
                id: 1,
                name: "Built-in".to_owned(),
                is_default: true,
            },
            OutputDevice {
                id: 2,
                name: "HDMI".to_owned(),
                is_default: false,
            },
        ];
        let next = vec![
            OutputDevice {
                id: 1,
                name: "Built-in".to_owned(),
                is_default: false,
            },
            OutputDevice {
                id: 2,
                name: "HDMI".to_owned(),
                is_default: true,
            },
        ];

        assert!(!output_structure_changed(&previous, &next));
    }

    #[test]
    fn output_structure_detects_identity_and_order_changes() {
        let previous = vec![OutputDevice {
            id: 1,
            name: "Built-in".to_owned(),
            is_default: true,
        }];
        let renamed = vec![OutputDevice {
            id: 1,
            name: "Desk".to_owned(),
            is_default: true,
        }];
        let added = vec![
            previous[0].clone(),
            OutputDevice {
                id: 2,
                name: "HDMI".to_owned(),
                is_default: false,
            },
        ];

        assert!(output_structure_changed(&previous, &renamed));
        assert!(output_structure_changed(&previous, &added));
        assert!(output_structure_changed(
            &added,
            &[added[1].clone(), added[0].clone()]
        ));
        assert!(output_structure_changed(&added, &[]));
    }

    #[test]
    fn popup_geometry_is_clamped_to_small_and_large_monitors() {
        assert_eq!(volume_popup_top_margin(), 8);
        assert_eq!(volume_popup_top_margin_for_height(400, 1080, 8), 8);
        assert_eq!(volume_popup_top_margin_for_height(1040, 1080, 8), 8);
        assert_eq!(volume_popup_top_margin_for_height(1200, 1080, 8), 8);
        assert_eq!(volume_popup_right_margin(1180, 1200, 260, 8), 8);
        assert_eq!(volume_popup_right_margin(600, 1920, 260, 8), 1190);
        assert_eq!(volume_popup_right_margin(100, 1920, 260, 8), 1652);
    }
}
