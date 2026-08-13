//! Composition of the three visual regions of Kbar.

use std::sync::mpsc;

use gtk::prelude::*;
use gtk4 as gtk;

use crate::services::VolumeAction;
use crate::ui::clock::ClockWidget;
use crate::ui::popover::PopoverCoordinator;
use crate::ui::status::StatusWidget;
use crate::ui::workspaces::WorkspacesWidget;
use kshell_theme::tokens;

pub struct BarUi {
    pub root: gtk::CenterBox,
    pub workspaces: WorkspacesWidget,
    pub clock: ClockWidget,
    pub status: StatusWidget,
}

impl BarUi {
    pub fn new(
        application: &gtk::Application,
        main_window: &gtk::ApplicationWindow,
        monitor: Option<gtk::gdk::Monitor>,
        coordinator: &PopoverCoordinator,
        output_name: Option<String>,
        action_sender: mpsc::Sender<VolumeAction>,
    ) -> Self {
        let workspaces = WorkspacesWidget::new(output_name);
        let clock = ClockWidget::new(coordinator);
        let status = StatusWidget::new(
            application,
            main_window,
            monitor,
            coordinator,
            action_sender,
        );

        let root = gtk::CenterBox::new();
        root.add_css_class("kbar");
        root.set_hexpand(true);
        root.set_height_request(tokens::BAR_HEIGHT);
        root.set_start_widget(Some(workspaces.widget()));
        root.set_center_widget(Some(clock.widget()));
        root.set_end_widget(Some(status.widget()));

        Self {
            root,
            workspaces,
            clock,
            status,
        }
    }
}
