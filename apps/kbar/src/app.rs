//! Kbar lifecycle, layer-shell setup and event-driven runtime.

use std::cell::Cell;
use std::env;
use std::error::Error;
use std::rc::Rc;

use futures_channel::mpsc::unbounded;
use futures_util::StreamExt;
use gtk::gdk;
use gtk::prelude::*;
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::services::{self, AudioStatus, BatteryStatus, NetworkStatus, StatusUpdate};
use crate::ui::bar::BarUi;
use crate::ui::popover::PopoverCoordinator;
use kshell_theme::tokens;

const APPLICATION_ID: &str = "io.github.ccmcorrea1.kshell.Bar";
const STYLE: &str = include_str!("ui/style.css");

#[derive(Clone, Debug)]
enum BarEvent {
    Audio(AudioStatus),
    Network(NetworkStatus),
    Battery(Option<BatteryStatus>),
    Workspaces(kshell_niri::WorkspaceState),
}

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

    let output_context = OutputContext::resolve();
    let window = gtk::ApplicationWindow::new(application);
    window.add_css_class("kbar-window");
    window.set_decorated(false);
    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_exclusive_zone(tokens::BAR_HEIGHT + tokens::BAR_MARGIN);
    window.set_namespace(Some(kshell_niri::BAR_NAMESPACE));
    output_context.apply(&window);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_margin(Edge::Top, tokens::BAR_MARGIN);
    window.set_margin(Edge::Left, tokens::BAR_MARGIN);
    window.set_margin(Edge::Right, tokens::BAR_MARGIN);

    let (status_sender, status_receiver) = unbounded();
    let volume_actions = services::spawn_status_worker(status_sender);
    let coordinator = PopoverCoordinator::new(&window);
    let bar = Rc::new(BarUi::new(
        application,
        &window,
        output_context.monitor.clone(),
        &coordinator,
        output_context.niri_name.clone(),
        volume_actions,
    ));
    bar.clock.start();
    window.set_child(Some(&bar.root));

    let (bar_sender, bar_receiver) = unbounded();
    let status_event_sender = bar_sender.clone();
    let main_context = gtk::glib::MainContext::default();
    let _status_bridge = main_context.spawn_local(async move {
        let mut receiver = status_receiver;
        while let Some(update) = receiver.next().await {
            let result = match update {
                StatusUpdate::Audio(audio) => {
                    status_event_sender.unbounded_send(BarEvent::Audio(audio))
                }
                StatusUpdate::SlowSystem(slow) => {
                    if status_event_sender
                        .unbounded_send(BarEvent::Network(slow.network))
                        .is_err()
                    {
                        return;
                    }
                    status_event_sender.unbounded_send(BarEvent::Battery(slow.battery))
                }
            };
            if result.is_err() {
                return;
            }
        }
    });

    let niri_event_sender = bar_sender.clone();
    let _niri_thread = kshell_niri::spawn_event_stream(move |state| {
        niri_event_sender
            .unbounded_send(BarEvent::Workspaces(state))
            .is_ok()
    });

    let bar_for_events = Rc::clone(&bar);
    let _event_dispatch = main_context.spawn_local(async move {
        let mut receiver = bar_receiver;
        while let Some(event) = receiver.next().await {
            match event {
                BarEvent::Audio(audio) => bar_for_events.status.update_audio(&audio),
                BarEvent::Network(network) => bar_for_events.status.update_network(network),
                BarEvent::Battery(battery) => bar_for_events.status.update_battery(battery),
                BarEvent::Workspaces(workspaces) => bar_for_events.workspaces.update(&workspaces),
            }
        }
    });

    let application = application.clone();
    window.connect_close_request(move |_| {
        application.quit();
        gtk::glib::Propagation::Proceed
    });

    window.present();
}

#[derive(Clone)]
struct OutputContext {
    monitor: Option<gdk::Monitor>,
    niri_name: Option<String>,
}

impl OutputContext {
    fn resolve() -> Self {
        let niri_name = env::var("KSHELL_OUTPUT")
            .ok()
            .filter(|name| !name.trim().is_empty());
        let monitor = niri_name.as_deref().and_then(find_monitor);
        Self { monitor, niri_name }
    }

    fn apply(&self, window: &gtk::ApplicationWindow) {
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
