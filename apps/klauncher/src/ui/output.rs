//! Best-effort output selection shared by the launcher window setup.

use std::env;

use gtk::gdk;
use gtk::prelude::*;
use gtk4 as gtk;
use gtk4_layer_shell::LayerShell;

pub(crate) struct OutputContext {
    pub(crate) monitor: Option<gdk::Monitor>,
}

impl OutputContext {
    pub(crate) fn resolve() -> Self {
        let monitor = env::var("KSHELL_OUTPUT")
            .ok()
            .filter(|name| !name.trim().is_empty())
            .and_then(|name| find_monitor(&name));
        Self { monitor }
    }

    pub(crate) fn apply(&self, window: &gtk::ApplicationWindow) {
        if let Some(monitor) = self.monitor.as_ref() {
            window.set_monitor(Some(monitor));
        }
    }
}

fn find_monitor(name: &str) -> Option<gdk::Monitor> {
    let display = gdk::Display::default()?;
    let monitors = display.monitors();
    (0..monitors.n_items()).find_map(|index| {
        let monitor = monitors.item(index)?.downcast::<gdk::Monitor>().ok()?;
        (monitor.connector().as_deref() == Some(name)).then_some(monitor)
    })
}
