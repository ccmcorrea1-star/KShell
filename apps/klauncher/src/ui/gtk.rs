use std::cell::{Cell, RefCell};
use std::error::Error;
use std::io;
use std::path::Path;
use std::rc::Rc;

use gtk::gdk;
use gtk::prelude::*;
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::core::{desktop::DesktopEntry, search};
use crate::ui::selection::{Direction, SelectionState};
use kshell_theme::tokens;

const APPLICATION_ID: &str = "com.klaucher.Launcher";
const LAYER_NAMESPACE: &str = kshell_niri::LAUNCHER_NAMESPACE;
const MAX_EMPTY_QUERY_CHARS: usize = 32;
const MAX_EMPTY_MESSAGE_WIDTH_CHARS: i32 = 60;
const STYLE: &str = include_str!("style.css");

pub fn run(applications: Rc<[DesktopEntry]>) -> Result<Option<usize>, Box<dyn Error>> {
    let selected = Rc::new(Cell::new(None));
    let unsupported = Rc::new(Cell::new(false));
    let application = gtk::Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    {
        let applications = Rc::clone(&applications);
        let selected = Rc::clone(&selected);
        let unsupported = Rc::clone(&unsupported);
        application.connect_activate(move |application| {
            if !gtk4_layer_shell::is_supported() {
                unsupported.set(true);
                eprintln!("gtk4-layer-shell is not supported by this Wayland display");
                application.quit();
                return;
            }

            build_launcher(application, applications.clone(), selected.clone());
        });
    }

    let exit_code = application.run();
    if unsupported.get() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "gtk4-layer-shell requires a Wayland compositor with layer-shell support",
        )
        .into());
    }
    if exit_code.get() != 0 {
        return Err(io::Error::other(format!(
            "GTK application exited with status {}",
            exit_code.get()
        ))
        .into());
    }

    Ok(selected.take())
}

fn build_launcher(
    application: &gtk::Application,
    applications: Rc<[DesktopEntry]>,
    selected: Rc<Cell<Option<usize>>>,
) {
    install_css();

    let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
    backdrop.add_css_class("launcher-backdrop");
    backdrop.set_hexpand(true);
    backdrop.set_vexpand(true);
    backdrop.set_can_target(false);

    let backdrop_revealer = gtk::Revealer::new();
    backdrop_revealer.set_child(Some(&backdrop));
    backdrop_revealer.set_reveal_child(false);
    backdrop_revealer.set_transition_type(gtk::RevealerTransitionType::Crossfade);
    let animation_duration = match gtk::Settings::default() {
        Some(settings) if settings.is_gtk_enable_animations() => tokens::BACKDROP_ANIMATION_MS,
        _ => 0,
    };
    backdrop_revealer.set_transition_duration(animation_duration);
    backdrop_revealer.set_hexpand(true);
    backdrop_revealer.set_vexpand(true);
    backdrop_revealer.set_can_target(false);

    let window = gtk::ApplicationWindow::new(application);
    window.add_css_class("launcher-window");
    window.set_decorated(false);
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_exclusive_zone(0);
    window.set_namespace(Some(LAYER_NAMESPACE));
    for edge in [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left] {
        window.set_anchor(edge, true);
    }

    let surface = gtk::CenterBox::new();
    surface.add_css_class("launcher-surface");
    surface.set_hexpand(true);
    surface.set_vexpand(true);

    let root = gtk::Overlay::new();
    root.add_css_class("launcher-surface");
    root.set_hexpand(true);
    root.set_vexpand(true);
    root.set_child(Some(&backdrop_revealer));
    root.add_overlay(&surface);

    let (panel_width, panel_height) = panel_size();
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.add_css_class("launcher-panel");
    panel.set_width_request(panel_width);
    panel.set_height_request(panel_height);
    panel.set_margin_start(tokens::PANEL_MARGIN);
    panel.set_margin_end(tokens::PANEL_MARGIN);
    panel.set_margin_top(tokens::PANEL_MARGIN);
    panel.set_margin_bottom(tokens::PANEL_MARGIN);
    panel.set_halign(gtk::Align::Center);
    panel.set_valign(gtk::Align::Center);
    surface.set_center_widget(Some(&panel));

    let search_header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    search_header.add_css_class("launcher-search-header");
    search_header.set_hexpand(true);

    let prompt = gtk::Label::new(Some(">"));
    prompt.add_css_class("launcher-prompt");
    prompt.set_margin_start(tokens::PANEL_INSET);
    prompt.set_margin_end(tokens::ICON_NAME_GAP);
    prompt.set_valign(gtk::Align::Center);
    search_header.append(&prompt);

    let search_entry = gtk::Entry::new();
    search_entry.set_placeholder_text(Some("search applications..."));
    search_entry.set_hexpand(true);
    search_entry.set_margin_end(tokens::PANEL_INSET);
    search_entry.set_valign(gtk::Align::Center);
    search_entry.add_css_class("launcher-search");
    search_header.append(&search_entry);

    panel.append(&search_header);

    let model = gtk::StringList::new(&[]);
    let selection = gtk::SingleSelection::new(Some(model.clone()));
    selection.set_autoselect(false);

    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, list_item| {
        let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
            return;
        };

        let row = gtk::Box::new(gtk::Orientation::Horizontal, tokens::ICON_NAME_GAP);
        row.set_margin_start(tokens::SPACE_2);
        row.set_margin_end(tokens::ICON_NAME_GAP);
        row.set_height_request(tokens::ROW_HEIGHT);

        let image = gtk::Image::new();
        image.set_pixel_size(tokens::ICON_SIZE);
        image.set_width_request(tokens::ICON_SIZE);
        row.append(&image);

        let label = gtk::Label::new(None);
        label.add_css_class("launcher-row-title");
        label.set_halign(gtk::Align::Start);
        label.set_hexpand(true);
        label.set_valign(gtk::Align::Center);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        row.append(&label);

        list_item.set_child(Some(&row));
    });

    let state = Rc::new(RefCell::new(SelectionState::new()));
    {
        let applications = Rc::clone(&applications);
        let state = Rc::clone(&state);
        factory.connect_bind(move |_, list_item| {
            let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                return;
            };

            let Some(row) = list_item.child().and_downcast::<gtk::Box>() else {
                return;
            };
            let Some(image) = row.first_child().and_downcast::<gtk::Image>() else {
                return;
            };
            let Some(label) = image.next_sibling().and_downcast::<gtk::Label>() else {
                return;
            };
            let application_index = {
                state
                    .borrow()
                    .application_index_at(list_item.position() as usize)
            };
            let Some(application_index) = application_index else {
                return;
            };

            let Some(application) = applications.get(application_index) else {
                return;
            };
            label.set_label(&application.name);
            set_application_icon(&image, application.icon.as_deref());
        });
    }

    let list_view = gtk::ListView::new(Some(selection.clone()), Some(factory));
    list_view.set_single_click_activate(true);
    list_view.add_css_class("launcher-list");

    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled.set_vexpand(true);
    scrolled.set_child(Some(&list_view));

    let placeholder = gtk::Label::new(Some("No applications found"));
    placeholder.add_css_class("launcher-placeholder");
    placeholder.set_halign(gtk::Align::Center);
    placeholder.set_valign(gtk::Align::Center);
    placeholder.set_hexpand(true);
    placeholder.set_max_width_chars(MAX_EMPTY_MESSAGE_WIDTH_CHARS);
    placeholder.set_wrap(true);
    placeholder.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    placeholder.set_justify(gtk::Justification::Center);
    let content = gtk::Overlay::new();
    content.set_hexpand(true);
    content.set_vexpand(true);
    content.set_child(Some(&scrolled));
    content.add_overlay(&placeholder);
    panel.append(&content);

    window.set_child(Some(&root));

    let finish = {
        let application = application.clone();
        let selected = Rc::clone(&selected);
        let window = window.clone();
        Rc::new(move |application_index: Option<usize>| {
            selected.set(application_index);
            window.close();
            application.quit();
        })
    };

    {
        let state = Rc::clone(&state);
        let finish = Rc::clone(&finish);
        list_view.connect_activate(move |_, position| {
            let application_index = { state.borrow().activate_row(position as usize) };
            finish(application_index);
        });
    }

    {
        let state = Rc::clone(&state);
        let finish = Rc::clone(&finish);
        search_entry.connect_activate(move |_| {
            if let Some(application_index) = state.borrow().activate_selected() {
                finish(Some(application_index));
            }
        });
    }

    {
        let finish = Rc::clone(&finish);
        let state = Rc::clone(&state);
        let selection = selection.clone();
        let list_view = list_view.clone();
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(move |_, key, _, _| match key {
            gdk::Key::Escape => {
                finish(None);
                gtk::glib::Propagation::Stop
            }
            gdk::Key::Up | gdk::Key::Down => {
                let direction = if key == gdk::Key::Up {
                    Direction::Up
                } else {
                    Direction::Down
                };
                let next = { state.borrow_mut().navigate(direction) };
                let Some(next) = next else {
                    return gtk::glib::Propagation::Stop;
                };

                selection.set_selected(next as u32);
                list_view.scroll_to(next as u32, gtk::ListScrollFlags::NONE, None);
                gtk::glib::Propagation::Stop
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                let application_index = { state.borrow().activate_selected() };
                finish(application_index);
                gtk::glib::Propagation::Stop
            }
            _ => gtk::glib::Propagation::Proceed,
        });
        search_entry.add_controller(key_controller);
    }

    {
        let applications = Rc::clone(&applications);
        let state = Rc::clone(&state);
        let model = model.clone();
        let selection = selection.clone();
        let placeholder = placeholder.clone();
        let update_results = Rc::new(move |query: &str| {
            let query = query.trim();
            let results = search::filter(&applications, query);
            let indices = results
                .iter()
                .map(|result| result.index)
                .collect::<Vec<_>>();
            let (result_count, selected_row) = {
                let mut state = state.borrow_mut();
                state.update_results(indices);

                (state.result_count(), state.selected_row())
            };

            let empty_items = vec![""; result_count];
            model.splice(0, model.n_items(), &empty_items);
            let empty_message = empty_message(query);
            placeholder.set_label(&empty_message);
            placeholder.set_visible(result_count == 0);
            selection.set_selected(selected_row.map(|row| row as u32).unwrap_or(u32::MAX));
        });

        let update_results_for_signal = Rc::clone(&update_results);
        search_entry.connect_changed(move |entry| {
            update_results_for_signal(entry.text().as_str());
        });
        update_results("");
    }

    {
        let window = window.clone();
        let panel = panel.clone();
        let surface_for_click = surface.clone();
        let click = gtk::GestureClick::new();
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        click.connect_pressed(move |_, _, x, y| {
            let Some(bounds) = panel.compute_bounds(&surface_for_click) else {
                return;
            };
            let inside_panel = x >= f64::from(bounds.x())
                && x < f64::from(bounds.x() + bounds.width())
                && y >= f64::from(bounds.y())
                && y < f64::from(bounds.y() + bounds.height());
            if !inside_panel {
                window.close();
            }
        });
        surface.add_controller(click);
    }

    {
        let application = application.clone();
        window.connect_close_request(move |_| {
            application.quit();
            gtk::glib::Propagation::Proceed
        });
    }

    window.present();
    let backdrop_revealer_for_animation = backdrop_revealer.clone();
    gtk::glib::idle_add_local_once(move || {
        backdrop_revealer_for_animation.set_reveal_child(true);
    });
    search_entry.grab_focus();
}

fn empty_message(query: &str) -> String {
    if query.is_empty() {
        return "No applications available".to_owned();
    }

    let mut characters = query.chars();
    let displayed_query = characters
        .by_ref()
        .take(MAX_EMPTY_QUERY_CHARS.saturating_sub(1))
        .collect::<String>();
    let displayed_query = if characters.next().is_some() {
        format!("{displayed_query}…")
    } else {
        displayed_query
    };

    format!("No applications found for \"{displayed_query}\"")
}

fn set_application_icon(image: &gtk::Image, icon: Option<&str>) {
    image.clear();
    let Some(icon) = icon.filter(|icon| !icon.is_empty()) else {
        return;
    };

    if Path::new(icon).is_absolute() {
        image.set_from_file(Some(icon));
    } else {
        image.set_icon_name(Some(icon));
    }
}

fn panel_size() -> (i32, i32) {
    let Some(display) = gdk::Display::default() else {
        return (tokens::PANEL_WIDTH, tokens::PANEL_HEIGHT);
    };
    let Some(monitor) = display.monitors().item(0).and_downcast::<gdk::Monitor>() else {
        return (tokens::PANEL_WIDTH, tokens::PANEL_HEIGHT);
    };

    let geometry = monitor.geometry();
    (
        tokens::PANEL_WIDTH.min((geometry.width() - tokens::PANEL_MARGIN * 2).max(1)),
        tokens::PANEL_HEIGHT.min((geometry.height() - tokens::PANEL_MARGIN * 2).max(1)),
    )
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
    use super::{empty_message, MAX_EMPTY_QUERY_CHARS};

    #[test]
    fn empty_message_uses_available_copy_without_a_query() {
        assert_eq!(empty_message(""), "No applications available");
    }

    #[test]
    fn empty_message_preserves_a_short_query() {
        assert_eq!(
            empty_message("calculator"),
            "No applications found for \"calculator\""
        );
    }

    #[test]
    fn empty_message_limits_long_queries_without_splitting_characters() {
        let query = "ação".repeat(MAX_EMPTY_QUERY_CHARS);
        let displayed_query = query
            .chars()
            .take(MAX_EMPTY_QUERY_CHARS - 1)
            .collect::<String>();

        assert_eq!(
            empty_message(&query),
            format!("No applications found for \"{displayed_query}…\"")
        );
    }
}
