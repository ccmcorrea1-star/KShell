use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use gtk::gdk;
use gtk::prelude::*;
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::{calendar, clock, niri, system};
use kshell_theme::tokens;

const APPLICATION_ID: &str = "com.klaucher.Bar";
const STYLE: &str = include_str!("style.css");

pub fn run() -> Result<(), Box<dyn Error>> {
    let unsupported = Rc::new(Cell::new(false));
    let application = gtk::Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    {
        let unsupported = Rc::clone(&unsupported);
        application.connect_activate(move |application| {
            if !gtk4_layer_shell::is_supported() {
                unsupported.set(true);
                application.quit();
                return;
            }

            build_bar(application);
        });
    }

    let exit_code = application.run();
    if unsupported.get() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "gtk4-layer-shell requires a Wayland compositor with layer-shell support",
        )
        .into());
    }
    if exit_code.get() != 0 {
        return Err(std::io::Error::other(format!(
            "GTK application exited with status {}",
            exit_code.get()
        ))
        .into());
    }

    Ok(())
}

fn build_bar(application: &gtk::Application) {
    install_css();

    let (workspace_box, workspace_buttons) = build_workspaces();
    let (status_sender, status_receiver) = mpsc::channel();
    let volume_actions = system::spawn_status_worker(status_sender);
    let status_widgets = Rc::new(build_status(volume_actions));

    let window = gtk::ApplicationWindow::new(application);
    window.add_css_class("kbar-window");
    window.set_decorated(false);
    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_exclusive_zone(tokens::BAR_HEIGHT + tokens::BAR_MARGIN);
    window.set_namespace(Some(kshell_niri::BAR_NAMESPACE));
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_margin(Edge::Top, tokens::BAR_MARGIN);
    window.set_margin(Edge::Left, tokens::BAR_MARGIN);
    window.set_margin(Edge::Right, tokens::BAR_MARGIN);

    let (clock_box, clock_date, clock_time) = build_clock(&window);

    let bar = gtk::CenterBox::new();
    bar.add_css_class("kbar");
    bar.set_hexpand(true);
    bar.set_height_request(tokens::BAR_HEIGHT);
    bar.set_start_widget(Some(&workspace_box));
    bar.set_center_widget(Some(&clock_box));
    bar.set_end_widget(Some(&status_widgets.container));

    window.set_child(Some(&bar));

    let (niri_sender, niri_receiver) = mpsc::channel();
    niri::spawn_event_stream(niri_sender);
    let workspace_buttons_for_updates = Rc::clone(&workspace_buttons);
    gtk::glib::timeout_add_local(Duration::from_millis(100), move || {
        while let Ok(state) = niri_receiver.try_recv() {
            apply_workspace_state(&workspace_buttons_for_updates, &state);
        }
        gtk::glib::ControlFlow::Continue
    });

    let status_widgets_for_updates = Rc::clone(&status_widgets);
    gtk::glib::timeout_add_local(Duration::from_millis(100), move || {
        while let Ok(status) = status_receiver.try_recv() {
            apply_system_status(&status_widgets_for_updates, status);
        }
        gtk::glib::ControlFlow::Continue
    });

    update_clock(&clock_date, &clock_time);
    gtk::glib::timeout_add_local(Duration::from_secs(1), move || {
        update_clock(&clock_date, &clock_time);
        gtk::glib::ControlFlow::Continue
    });

    {
        let application = application.clone();
        window.connect_close_request(move |_| {
            application.quit();
            gtk::glib::Propagation::Proceed
        });
    }

    window.present();
}

fn build_workspaces() -> (gtk::Box, Rc<Vec<gtk::Button>>) {
    let container = gtk::Box::new(gtk::Orientation::Horizontal, tokens::WORKSPACE_GAP);
    container.add_css_class("kbar-workspaces");
    container.set_valign(gtk::Align::Center);

    let mut buttons = Vec::with_capacity(niri::WORKSPACE_COUNT);
    for index in 1..=niri::WORKSPACE_COUNT {
        let button = gtk::Button::with_label(&index.to_string());
        button.add_css_class("kbar-workspace");
        button.set_width_request(tokens::WORKSPACE_SIZE);
        button.set_height_request(tokens::WORKSPACE_SIZE);
        button.set_tooltip_text(Some(&format!("Workspace {index}")));
        button.connect_clicked(move |_| niri::focus_workspace(index));
        container.append(&button);
        buttons.push(button);
    }

    (container, Rc::new(buttons))
}

fn build_clock(window: &gtk::ApplicationWindow) -> (gtk::Box, gtk::Label, gtk::Label) {
    let container = gtk::Box::new(gtk::Orientation::Horizontal, tokens::CLOCK_DIVIDER_GAP);
    container.add_css_class("kbar-clock");
    container.set_valign(gtk::Align::Center);
    container.set_halign(gtk::Align::Center);
    container.set_focusable(true);
    container.set_tooltip_text(Some("Calendário"));

    let date = gtk::Label::new(None);
    date.add_css_class("kbar-clock-date");
    let divider = gtk::Label::new(Some("•"));
    divider.add_css_class("kbar-clock-divider");
    let time = gtk::Label::new(None);
    time.add_css_class("kbar-clock-time");

    container.append(&date);
    container.append(&divider);
    container.append(&time);
    build_calendar_popover(&container, window);
    (container, date, time)
}

#[derive(Clone)]
struct CalendarView {
    today: Rc<Cell<Option<calendar::Date>>>,
    displayed_month: Rc<Cell<calendar::Date>>,
    month_label: gtk::Label,
    day_labels: Rc<Vec<gtk::Label>>,
}

fn build_calendar_popover(anchor: &gtk::Box, window: &gtk::ApplicationWindow) {
    let popover = gtk::Popover::new();
    popover.add_css_class("kbar-popover");
    popover.add_css_class("kbar-calendar-popover");
    popover.set_autohide(true);
    popover.set_has_arrow(false);
    popover.set_position(gtk::PositionType::Bottom);
    popover.set_parent(anchor);

    let window_for_close = window.clone();
    popover.connect_closed(move |_| {
        window_for_close.set_keyboard_mode(KeyboardMode::None);
    });

    let initial_today = calendar::today();
    let initial_month = initial_today
        .unwrap_or(calendar::Date::new(1970, 1, 1))
        .start_of_month();

    let content = gtk::Box::new(gtk::Orientation::Vertical, tokens::SPACE_3);
    content.add_css_class("kbar-calendar-content");
    content.set_width_request(tokens::CALENDAR_POPOVER_WIDTH);
    content.set_margin_top(tokens::SPACE_2);
    content.set_margin_bottom(tokens::SPACE_2);
    content.set_margin_start(tokens::SPACE_2);
    content.set_margin_end(tokens::SPACE_2);
    content.set_focusable(true);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, tokens::SPACE_2);
    header.add_css_class("kbar-calendar-header");
    header.set_valign(gtk::Align::Center);

    let previous_button = gtk::Button::with_label("‹");
    previous_button.add_css_class("kbar-calendar-nav");
    previous_button.set_width_request(tokens::WORKSPACE_SIZE);
    previous_button.set_height_request(tokens::WORKSPACE_SIZE);
    previous_button.set_tooltip_text(Some("Mês anterior"));

    let month_label = gtk::Label::new(None);
    month_label.add_css_class("kbar-calendar-title");
    month_label.set_halign(gtk::Align::Start);
    month_label.set_hexpand(true);

    let next_button = gtk::Button::with_label("›");
    next_button.add_css_class("kbar-calendar-nav");
    next_button.set_width_request(tokens::WORKSPACE_SIZE);
    next_button.set_height_request(tokens::WORKSPACE_SIZE);
    next_button.set_tooltip_text(Some("Próximo mês"));

    header.append(&previous_button);
    header.append(&month_label);
    header.append(&next_button);
    content.append(&header);

    let day_grid = gtk::Grid::new();
    day_grid.add_css_class("kbar-calendar-grid");
    day_grid.set_column_homogeneous(true);
    day_grid.set_row_homogeneous(true);
    day_grid.set_column_spacing(tokens::SPACE_1 as u32);
    day_grid.set_row_spacing(tokens::SPACE_1 as u32);

    for (column, weekday) in calendar::WEEKDAYS.iter().enumerate() {
        let label = gtk::Label::new(Some(weekday));
        label.add_css_class("kbar-calendar-weekday");
        label.set_halign(gtk::Align::Center);
        label.set_valign(gtk::Align::Center);
        day_grid.attach(&label, column as i32, 0, 1, 1);
    }

    let mut day_labels = Vec::with_capacity(calendar::GRID_SIZE);
    for index in 0..calendar::GRID_SIZE {
        let label = gtk::Label::new(None);
        label.add_css_class("kbar-calendar-day");
        label.set_halign(gtk::Align::Center);
        label.set_valign(gtk::Align::Center);
        label.set_size_request(tokens::CALENDAR_DAY_SIZE, tokens::CALENDAR_DAY_SIZE);
        label.set_single_line_mode(true);
        day_grid.attach(
            &label,
            (index % calendar::WEEKDAYS.len()) as i32,
            (index / calendar::WEEKDAYS.len() + 1) as i32,
            1,
            1,
        );
        day_labels.push(label);
    }
    content.append(&day_grid);

    let view = CalendarView {
        today: Rc::new(Cell::new(initial_today)),
        displayed_month: Rc::new(Cell::new(initial_month)),
        month_label,
        day_labels: Rc::new(day_labels),
    };
    refresh_calendar(&view);

    let view_for_previous = view.clone();
    previous_button.connect_clicked(move |_| {
        view_for_previous
            .displayed_month
            .set(view_for_previous.displayed_month.get().previous_month());
        refresh_calendar(&view_for_previous);
    });

    let view_for_next = view.clone();
    next_button.connect_clicked(move |_| {
        view_for_next
            .displayed_month
            .set(view_for_next.displayed_month.get().next_month());
        refresh_calendar(&view_for_next);
    });

    popover.set_child(Some(&content));

    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let popover_for_escape = popover.clone();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            popover_for_escape.popdown();
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    popover.add_controller(key_controller);

    let content_for_focus = content.clone();
    popover.connect_show(move |_| {
        content_for_focus.grab_focus();
    });

    let left_click = gtk::GestureClick::new();
    left_click.set_button(gdk::BUTTON_PRIMARY);
    left_click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let view_for_click = view.clone();
    let popover_for_click = popover.clone();
    let window_for_click = window.clone();
    left_click.connect_pressed(move |_, press_count, _, _| {
        if press_count == 1 {
            if let Some(today) = calendar::today() {
                view_for_click.today.set(Some(today));
                view_for_click.displayed_month.set(today.start_of_month());
            }
            refresh_calendar(&view_for_click);
            window_for_click.set_keyboard_mode(KeyboardMode::OnDemand);
            popover_for_click.popup();
        }
    });
    anchor.add_controller(left_click);
}

fn refresh_calendar(view: &CalendarView) {
    let month = view.displayed_month.get();
    view.month_label.set_label(&calendar::month_title(month));

    for (label, day) in view.day_labels.iter().zip(calendar::month_grid(month)) {
        label.set_label(&day.date.day.to_string());
        label.set_tooltip_text(Some(&calendar::date_label(day.date)));
        label.remove_css_class("is-outside-month");
        label.remove_css_class("is-today");
        if !day.current_month {
            label.add_css_class("is-outside-month");
        }
        if view.today.get() == Some(day.date) {
            label.add_css_class("is-today");
        }
    }
}

#[derive(Clone)]
struct StatusWidgets {
    container: gtk::Box,
    volume: VolumeWidgets,
    network_icon: StatusIcon,
    network_item: gtk::Box,
    battery_icon: StatusIcon,
    battery_label: gtk::Label,
    battery_item: gtk::Box,
}

#[derive(Clone)]
struct VolumeWidgets {
    volume_icon: StatusIcon,
    volume_label: gtk::Label,
    popover_icon: StatusIcon,
    popover_percent: gtk::Label,
    volume_scale: gtk::Scale,
    dragging_scale: Rc<Cell<bool>>,
    pending_scale_value: Rc<Cell<Option<u8>>>,
    waiting_for_sync: Rc<Cell<Option<u64>>>,
    output_empty_label: gtk::Label,
    output_list: gtk::ListBox,
    rendered_output_menu: Rc<RefCell<Option<OutputMenuState>>>,
    action_sender: mpsc::Sender<system::VolumeAction>,
}

#[derive(Clone)]
struct ScaleInteraction {
    scale: gtk::Scale,
    dragging: Rc<Cell<bool>>,
    pending_value: Rc<Cell<Option<u8>>>,
    pending_set_value: Rc<Cell<Option<u8>>>,
    set_timer: Rc<RefCell<Option<gtk::glib::SourceId>>>,
    waiting_for_sync: Rc<Cell<Option<u64>>>,
    next_sync_token: Rc<Cell<u64>>,
    action_sender: mpsc::Sender<system::VolumeAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OutputMenuState {
    current_output_id: Option<u32>,
    outputs: Vec<system::OutputDevice>,
}

fn build_status(action_sender: mpsc::Sender<system::VolumeAction>) -> StatusWidgets {
    let container = gtk::Box::new(gtk::Orientation::Horizontal, tokens::STATUS_GAP);
    container.add_css_class("kbar-status");
    container.set_valign(gtk::Align::Center);
    container.set_baseline_position(gtk::BaselinePosition::Center);

    let (volume_item, volume) = build_volume(action_sender);

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

    StatusWidgets {
        container,
        volume,
        network_icon,
        network_item,
        battery_icon,
        battery_label,
        battery_item,
    }
}

fn build_volume(action_sender: mpsc::Sender<system::VolumeAction>) -> (gtk::Box, VolumeWidgets) {
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
    // The volume item is centered inside the 32px bar, so Bottom would place
    // the popover below the item but still inside the bar. Move it past the
    // remaining half-height of the bar, plus a quiet 8px separation.
    popover.set_offset(
        0,
        (tokens::BAR_HEIGHT - tokens::STATUS_ICON_SIZE) / 2 + tokens::SPACE_2,
    );
    popover.set_parent(&volume_item);

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
        let _ = mute_action_sender.send(system::VolumeAction::ToggleMute);
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
    let dragging_scale = Rc::new(Cell::new(false));
    let pending_scale_value = Rc::new(Cell::new(None));
    let pending_set_value = Rc::new(Cell::new(None));
    let set_timer = Rc::new(RefCell::new(None));
    let waiting_for_sync = Rc::new(Cell::new(None));
    let next_sync_token = Rc::new(Cell::new(0));
    let scale_interaction = ScaleInteraction {
        scale: volume_scale.clone(),
        dragging: Rc::clone(&dragging_scale),
        pending_value: Rc::clone(&pending_scale_value),
        pending_set_value: Rc::clone(&pending_set_value),
        set_timer: Rc::clone(&set_timer),
        waiting_for_sync: Rc::clone(&waiting_for_sync),
        next_sync_token: Rc::clone(&next_sync_token),
        action_sender: action_sender.clone(),
    };
    let interaction_for_change = scale_interaction.clone();
    volume_scale.connect_change_value(move |_, _, value| {
        let percent = value.round().clamp(0.0, 100.0) as u8;
        // This signal is emitted by GtkRange for a user-driven change. Keep
        // the protection tied to it instead of relying on a separate click
        // gesture that can be cancelled once the pointer starts dragging.
        interaction_for_change.dragging.set(true);
        interaction_for_change.pending_value.set(Some(percent));
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
    left_click.connect_pressed(move |_, press_count, _, _| {
        if press_count == 1 {
            popover_for_click.popup();
        }
    });
    volume_item.add_controller(left_click);

    let middle_click = gtk::GestureClick::new();
    middle_click.set_button(gdk::BUTTON_MIDDLE);
    middle_click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let middle_action_sender = action_sender.clone();
    middle_click.connect_pressed(move |_, _, _, _| {
        let _ = middle_action_sender.send(system::VolumeAction::ToggleMute);
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
        let _ = scroll_action_sender.send(system::VolumeAction::Adjust(step));
        gtk::glib::Propagation::Stop
    });
    volume_item.add_controller(scroll);

    (
        volume_item,
        VolumeWidgets {
            volume_icon,
            volume_label,
            popover_icon,
            popover_percent,
            volume_scale,
            dragging_scale: scale_interaction.dragging,
            pending_scale_value: scale_interaction.pending_value,
            waiting_for_sync: scale_interaction.waiting_for_sync,
            output_empty_label,
            output_list,
            rendered_output_menu: Rc::new(RefCell::new(None)),
            action_sender,
        },
    )
}

fn scale_percent(scale: &gtk::Scale) -> u8 {
    scale.value().round().clamp(0.0, 100.0) as u8
}

impl ScaleInteraction {
    fn begin(&self) {
        self.dragging.set(true);
        self.pending_value.set(Some(scale_percent(&self.scale)));
    }

    fn finish(&self) {
        if !self.dragging.replace(false) {
            return;
        }

        let percent = scale_percent(&self.scale);
        self.pending_value.set(Some(percent));

        let token = self.next_sync_token.get().wrapping_add(1);
        self.next_sync_token.set(token);
        self.waiting_for_sync.set(Some(token));

        flush_volume_set(
            &self.pending_set_value,
            &self.set_timer,
            percent,
            &self.action_sender,
        );
        let _ = self.action_sender.send(system::VolumeAction::Sync {
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
    action_sender: &mpsc::Sender<system::VolumeAction>,
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
            let _ = action_sender_for_timer.send(system::VolumeAction::Set(percent));
        }
        gtk::glib::ControlFlow::Break
    });
    *timer.borrow_mut() = Some(source_id);
}

fn flush_volume_set(
    pending_value: &Cell<Option<u8>>,
    timer: &RefCell<Option<gtk::glib::SourceId>>,
    percent: u8,
    action_sender: &mpsc::Sender<system::VolumeAction>,
) {
    if let Some(source_id) = timer.borrow_mut().take() {
        source_id.remove();
    }
    pending_value.set(None);
    let _ = action_sender.send(system::VolumeAction::Set(percent));
}

fn status_item(icon: &StatusIcon, label: Option<&gtk::Label>) -> gtk::Box {
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

fn update_clock(date: &gtk::Label, time: &gtk::Label) {
    let text = clock::now();
    date.set_label(&text.date);
    time.set_label(&text.time);
}

fn apply_workspace_state(buttons: &[gtk::Button], state: &niri::WorkspaceState) {
    for (index, button) in buttons.iter().enumerate() {
        if state.focused_index == Some(index) {
            button.add_css_class("is-active");
        } else {
            button.remove_css_class("is-active");
        }
    }
}

fn apply_system_status(widgets: &StatusWidgets, status: system::SystemStatus) {
    apply_volume_status(&widgets.volume, &status.volume, status.volume_sync_token);

    widgets.network_icon.set_state(IconState::Network {
        connected: status.network_connected,
    });
    widgets
        .network_item
        .set_tooltip_text(Some(if status.network_connected {
            "Rede conectada"
        } else {
            "Rede desconectada"
        }));

    if let Some(battery) = status.battery {
        widgets
            .battery_label
            .set_label(&format!("{}%", battery.percent));
        widgets.battery_icon.set_state(IconState::Battery {
            percent: battery.percent,
            charging: battery.charging,
        });
        widgets.battery_item.set_visible(true);
    } else {
        widgets.battery_item.set_visible(false);
    }
}

fn apply_volume_status(
    widgets: &VolumeWidgets,
    status: &system::VolumeStatus,
    sync_token: Option<u64>,
) {
    if sync_token.is_some() && widgets.waiting_for_sync.get() == sync_token {
        widgets.waiting_for_sync.set(None);
        widgets.pending_scale_value.set(None);
    }

    let preserve_local_value =
        widgets.dragging_scale.get() || widgets.waiting_for_sync.get().is_some();
    let displayed_percent = volume_percent_for_display(
        widgets.pending_scale_value.get(),
        widgets.dragging_scale.get(),
        widgets.waiting_for_sync.get().is_some(),
        status.percent,
    );
    let volume_label = displayed_percent
        .map(|percent| format!("{percent}%"))
        .unwrap_or_else(|| "—%".to_owned());
    widgets.volume_label.set_label(&volume_label);
    widgets.popover_percent.set_label(&volume_label);

    let volume_state = IconState::Volume {
        percent: displayed_percent,
        muted: status.muted,
    };
    widgets.volume_icon.set_state(volume_state);
    widgets.popover_icon.set_state(volume_state);

    if let Some(percent) = displayed_percent {
        widgets.volume_scale.set_sensitive(true);
        if !preserve_local_value {
            widgets.volume_scale.set_value(f64::from(percent));
        }
    } else {
        widgets.volume_scale.set_sensitive(false);
    }

    let current_output_id = status.current_output.as_ref().map(|output| output.id);
    let has_outputs = !status.outputs.is_empty();
    widgets.output_empty_label.set_visible(!has_outputs);
    widgets
        .output_list
        .set_visible(should_show_output_list(status.outputs.len()));

    let output_menu_state = OutputMenuState {
        current_output_id,
        outputs: status.outputs.clone(),
    };
    let menu_changed = widgets.rendered_output_menu.borrow().as_ref() != Some(&output_menu_state);
    if !menu_changed {
        return;
    }

    widgets.output_list.remove_all();
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

        let action_sender = widgets.action_sender.clone();
        let output_id = output.id;
        button.connect_clicked(move |_| {
            let _ = action_sender.send(system::VolumeAction::SetDefault(output_id));
        });
        widgets.output_list.append(&button);
    }
    *widgets.rendered_output_menu.borrow_mut() = Some(output_menu_state);
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

#[derive(Clone, Copy)]
enum IconKind {
    Volume,
    Network,
    Battery,
}

#[derive(Clone, Copy)]
enum IconState {
    Volume { percent: Option<u8>, muted: bool },
    Network { connected: bool },
    Battery { percent: u8, charging: bool },
}

#[derive(Clone)]
struct StatusIcon {
    area: gtk::DrawingArea,
    state: Rc<Cell<IconState>>,
}

impl StatusIcon {
    fn new(kind: IconKind) -> Self {
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

    fn set_state(&self, state: IconState) {
        self.state.set(state);
        self.area.queue_draw();
    }
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

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(STYLE);

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{should_show_output_list, volume_level, volume_percent_for_display, VolumeLevel};

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
    fn shows_output_choices_when_outputs_exist() {
        assert!(!should_show_output_list(0));
        assert!(should_show_output_list(1));
        assert!(should_show_output_list(2));
    }
}
