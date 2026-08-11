//! Clock labels and minute-aligned scheduling.

use std::time::SystemTime;

use gtk::prelude::*;
use gtk4 as gtk;

use crate::clock;
use crate::ui::calendar::CalendarWidget;
use crate::ui::popover::PopoverCoordinator;
use kshell_theme::tokens;

pub struct ClockWidget {
    container: gtk::Box,
    date: gtk::Label,
    time: gtk::Label,
    _calendar: CalendarWidget,
}

impl ClockWidget {
    pub fn new(coordinator: &PopoverCoordinator) -> Self {
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
        let calendar = CalendarWidget::new(&container, coordinator);

        Self {
            container,
            date,
            time,
            _calendar: calendar,
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    pub fn start(&self) {
        update_labels(&self.date, &self.time);
        schedule_update(self.date.clone(), self.time.clone());
    }
}

fn schedule_update(date: gtk::Label, time: gtk::Label) {
    let delay = clock::duration_until_next_minute(SystemTime::now());
    gtk::glib::timeout_add_local(delay, move || {
        update_labels(&date, &time);
        schedule_update(date.clone(), time.clone());
        gtk::glib::ControlFlow::Break
    });
}

fn update_labels(date: &gtk::Label, time: &gtk::Label) {
    let text = clock::now();
    if date.text().as_str() != text.date {
        date.set_label(&text.date);
    }
    if time.text().as_str() != text.time {
        time.set_label(&text.time);
    }
}
