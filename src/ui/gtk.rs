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

const APPLICATION_ID: &str = "com.klaucher.Launcher";
const LAYER_NAMESPACE: &str = "my-shell-launcher";
const PANEL_WIDTH: i32 = 560;
const PANEL_HEIGHT: i32 = 420;
const ICON_SIZE: i32 = 32;

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

    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.add_css_class("launcher-panel");
    panel.set_width_request(PANEL_WIDTH);
    panel.set_height_request(PANEL_HEIGHT);
    panel.set_halign(gtk::Align::Center);
    panel.set_valign(gtk::Align::Center);
    surface.set_center_widget(Some(&panel));

    let search_entry = gtk::SearchEntry::new();
    search_entry.set_placeholder_text(Some("Search applications..."));
    search_entry.set_hexpand(true);
    search_entry.set_margin_start(16);
    search_entry.set_margin_end(16);
    search_entry.set_margin_top(12);
    search_entry.set_margin_bottom(12);
    search_entry.add_css_class("launcher-search");
    panel.append(&search_entry);
    panel.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let model = gtk::StringList::new(&[]);
    let selection = gtk::SingleSelection::new(Some(model.clone()));
    selection.set_autoselect(false);

    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, list_item| {
        let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
            return;
        };

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.set_margin_start(12);
        row.set_margin_end(12);
        row.set_margin_top(5);
        row.set_margin_bottom(5);

        let image = gtk::Image::new();
        image.set_pixel_size(ICON_SIZE);
        image.set_width_request(ICON_SIZE);
        row.append(&image);

        let label = gtk::Label::new(None);
        label.set_halign(gtk::Align::Start);
        label.set_hexpand(true);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        row.append(&label);

        list_item.set_child(Some(&row));
    });

    let result_indices = Rc::new(RefCell::new(Vec::<usize>::new()));
    {
        let applications = Rc::clone(&applications);
        let result_indices = Rc::clone(&result_indices);
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
            let Some(label) = row.last_child().and_downcast::<gtk::Label>() else {
                return;
            };
            let Some(application_index) = result_indices
                .borrow()
                .get(list_item.position() as usize)
                .copied()
            else {
                return;
            };

            let application = &applications[application_index];
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
    let content = gtk::Overlay::new();
    content.set_hexpand(true);
    content.set_vexpand(true);
    content.set_child(Some(&scrolled));
    content.add_overlay(&placeholder);
    panel.append(&content);

    window.set_child(Some(&surface));

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
        let result_indices = Rc::clone(&result_indices);
        let finish = Rc::clone(&finish);
        list_view.connect_activate(move |_, position| {
            let application_index = result_indices.borrow().get(position as usize).copied();
            finish(application_index);
        });
    }

    {
        let result_indices = Rc::clone(&result_indices);
        let finish = Rc::clone(&finish);
        let selection = selection.clone();
        search_entry.connect_activate(move |_| {
            let application_index = result_indices
                .borrow()
                .get(selection.selected() as usize)
                .copied();
            finish(application_index);
        });
    }

    {
        let finish = Rc::clone(&finish);
        let selection = selection.clone();
        let list_view = list_view.clone();
        let result_indices = Rc::clone(&result_indices);
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(move |_, key, _, _| match key {
            gdk::Key::Escape => {
                finish(None);
                gtk::glib::Propagation::Stop
            }
            gdk::Key::Up | gdk::Key::Down => {
                let count = result_indices.borrow().len();
                if count == 0 {
                    return gtk::glib::Propagation::Stop;
                }

                let current = selection.selected() as usize;
                let next = if current >= count {
                    0
                } else if key == gdk::Key::Up {
                    current.checked_sub(1).unwrap_or(count - 1)
                } else {
                    (current + 1) % count
                };
                selection.set_selected(next as u32);
                list_view.scroll_to(next as u32, gtk::ListScrollFlags::NONE, None);
                gtk::glib::Propagation::Stop
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                let application_index = result_indices
                    .borrow()
                    .get(selection.selected() as usize)
                    .copied();
                finish(application_index);
                gtk::glib::Propagation::Stop
            }
            _ => gtk::glib::Propagation::Proceed,
        });
        search_entry.add_controller(key_controller);
    }

    {
        let applications = Rc::clone(&applications);
        let result_indices = Rc::clone(&result_indices);
        let model = model.clone();
        let selection = selection.clone();
        let placeholder = placeholder.clone();
        let update_results = Rc::new(move |query: &str| {
            let results = search::filter(&applications, query);
            {
                let mut indices = result_indices.borrow_mut();
                indices.clear();
                indices.extend(results.iter().map(|result| result.index));
            }

            let empty_items = vec![""; results.len()];
            model.splice(0, model.n_items(), &empty_items);
            let has_results = model.n_items() != 0;
            placeholder.set_visible(!has_results);
            selection.set_selected(if has_results { 0 } else { u32::MAX });
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
    search_entry.grab_focus();
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

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(
        r#"
        .launcher-window {
            background-color: transparent;
        }

        .launcher-surface {
            background-color: transparent;
        }

        .launcher-panel {
            background-color: alpha(@window_bg_color, 0.98);
            border: 1px solid alpha(@borders, 0.75);
            border-radius: 12px;
            box-shadow: 0 8px 28px alpha(black, 0.35);
        }

        .launcher-search {
            background-color: transparent;
            border: none;
            box-shadow: none;
            font-size: 16px;
        }

        .launcher-list {
            background-color: transparent;
            padding: 8px;
        }

        .launcher-list row {
            border-radius: 7px;
        }

        .launcher-list row:selected {
            background-color: alpha(@accent_bg_color, 0.28);
        }

        .launcher-placeholder {
            color: alpha(@window_fg_color, 0.6);
            margin: 24px;
        }
        "#,
    );

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
