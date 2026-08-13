//! Calendar popover presentation. Date arithmetic remains in `crate::calendar`.

use std::cell::Cell;
use std::rc::Rc;

use gtk::gdk;
use gtk::prelude::*;
use gtk4 as gtk;

use crate::calendar::{self, Date};
use crate::ui::popover::{PopoverCoordinator, PopoverId};
use kshell_theme::tokens;

#[derive(Clone)]
struct CalendarView {
    today: Rc<Cell<Option<Date>>>,
    displayed_month: Rc<Cell<Date>>,
    month_label: gtk::Label,
    day_labels: Rc<Vec<gtk::Label>>,
}

pub struct CalendarWidget {
    _popover: gtk::Popover,
}

impl CalendarWidget {
    pub fn new(anchor: &gtk::Box, coordinator: &PopoverCoordinator) -> Self {
        let popover = gtk::Popover::new();
        popover.add_css_class("kbar-popover");
        popover.add_css_class("kbar-calendar-popover");
        // Keep Calendar as a GtkPopover, but let a click on another bar
        // module reach that module. Focus leave and the explicit Escape
        // handler below retain dismissal without consuming that click.
        popover.set_autohide(false);
        popover.set_has_arrow(false);
        popover.set_position(gtk::PositionType::Bottom);
        popover.set_offset(
            0,
            (tokens::BAR_HEIGHT - tokens::STATUS_ICON_SIZE) / 2 + tokens::SPACE_2,
        );
        popover.set_parent(anchor);
        let popover_for_coordinator = popover.clone();
        coordinator.register(PopoverId::Calendar, move || {
            popover_for_coordinator.popdown();
        });

        let coordinator_for_close = coordinator.clone();
        popover.connect_closed(move |_| coordinator_for_close.close(PopoverId::Calendar));

        let initial_today = calendar::today();
        let initial_month = initial_today
            .unwrap_or(Date::new(1970, 1, 1))
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

        let focus_controller = gtk::EventControllerFocus::new();
        let popover_for_focus = popover.clone();
        focus_controller.connect_leave(move |_| {
            if popover_for_focus.is_visible() {
                popover_for_focus.popdown();
            }
        });
        popover.add_controller(focus_controller);

        let content_for_focus = content.clone();
        let view_for_show = view.clone();
        popover.connect_show(move |_| {
            content_for_focus.grab_focus();
            if let Some(today) = calendar::today() {
                if view_for_show.today.get() != Some(today) {
                    view_for_show.today.set(Some(today));
                    view_for_show.displayed_month.set(today.start_of_month());
                    refresh_calendar(&view_for_show);
                }
            }
        });

        let left_click = gtk::GestureClick::new();
        left_click.set_button(gdk::BUTTON_PRIMARY);
        left_click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let view_for_click = view.clone();
        let popover_for_click = popover.clone();
        let coordinator_for_click = coordinator.clone();
        left_click.connect_pressed(move |_, press_count, _, _| {
            if press_count == 1 {
                if popover_for_click.is_visible() {
                    popover_for_click.popdown();
                    return;
                }
                if let Some(today) = calendar::today() {
                    view_for_click.today.set(Some(today));
                    view_for_click.displayed_month.set(today.start_of_month());
                }
                refresh_calendar(&view_for_click);
                coordinator_for_click.open(PopoverId::Calendar);
                popover_for_click.popup();
            }
        });
        anchor.add_controller(left_click);

        Self { _popover: popover }
    }
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
