//! Workspace buttons. The five visible slots are a UI choice, not Niri state.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use kshell_niri::WorkspaceState;
use kshell_theme::tokens;

pub const DEFAULT_VISIBLE_WORKSPACE_SLOTS: usize = 5;

pub struct WorkspacesWidget {
    container: gtk::Box,
    buttons: Vec<gtk::Button>,
    output_name: Option<String>,
    last_active: Cell<Option<usize>>,
    state: Rc<RefCell<Option<WorkspaceState>>>,
}

impl WorkspacesWidget {
    pub fn new(output_name: Option<String>) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, tokens::WORKSPACE_GAP);
        container.add_css_class("kbar-workspaces");
        container.set_valign(gtk::Align::Center);

        let mut buttons = Vec::with_capacity(DEFAULT_VISIBLE_WORKSPACE_SLOTS);
        let state: Rc<RefCell<Option<WorkspaceState>>> = Rc::new(RefCell::new(None));
        for index in 1..=DEFAULT_VISIBLE_WORKSPACE_SLOTS {
            let button = gtk::Button::with_label(&index.to_string());
            button.add_css_class("kbar-workspace");
            button.set_width_request(tokens::WORKSPACE_SIZE);
            button.set_height_request(tokens::WORKSPACE_SIZE);
            button.set_tooltip_text(Some(&format!("Workspace {index}")));
            let state_for_click = Rc::clone(&state);
            let output_for_click = output_name.clone();
            button.connect_clicked(move |_| {
                let target_id = state_for_click.borrow().as_ref().and_then(|state| {
                    output_for_click.as_deref().and_then(|output| {
                        state
                            .workspaces
                            .iter()
                            .find(|workspace| {
                                workspace.output.as_deref() == Some(output)
                                    && workspace.index == index
                            })
                            .map(|workspace| workspace.id)
                    })
                });
                if let Some(target_id) = target_id {
                    kshell_niri::focus_workspace_id(target_id);
                } else {
                    kshell_niri::focus_workspace(index);
                }
            });
            container.append(&button);
            buttons.push(button);
        }

        Self {
            container,
            buttons,
            output_name,
            last_active: Cell::new(None),
            state,
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    pub fn update(&self, state: &WorkspaceState) {
        *self.state.borrow_mut() = Some(state.clone());
        let active = active_slot(state, self.output_name.as_deref());
        if self.last_active.get() == active {
            return;
        }

        for (index, button) in self.buttons.iter().enumerate() {
            if active == Some(index) {
                button.add_css_class("is-active");
            } else {
                button.remove_css_class("is-active");
            }
        }
        self.last_active.set(active);
    }
}

fn active_slot(state: &WorkspaceState, output_name: Option<&str>) -> Option<usize> {
    let index = output_name
        .and_then(|output| state.active_index_for(output))
        .or_else(|| state.focused_workspace().map(|workspace| workspace.index))?;
    index
        .checked_sub(1)
        .filter(|index| *index < DEFAULT_VISIBLE_WORKSPACE_SLOTS)
}

#[cfg(test)]
mod tests {
    use super::{active_slot, DEFAULT_VISIBLE_WORKSPACE_SLOTS};
    use kshell_niri::{Workspace, WorkspaceState};

    fn workspace(index: usize, output: &str, active: bool, focused: bool) -> Workspace {
        Workspace {
            id: index as u64,
            index,
            name: None,
            output: Some(output.to_owned()),
            is_urgent: false,
            is_active: active,
            is_focused: focused,
            active_window_id: None,
        }
    }

    #[test]
    fn five_visual_slots_are_independent_from_compositor_workspace_count() {
        assert_eq!(DEFAULT_VISIBLE_WORKSPACE_SLOTS, 5);
        let state = WorkspaceState {
            workspaces: vec![workspace(7, "A", true, true)],
            focused_output: Some("A".to_owned()),
        };
        assert_eq!(active_slot(&state, Some("A")), None);
    }

    #[test]
    fn output_context_selects_active_workspace_before_global_focus() {
        let state = WorkspaceState {
            workspaces: vec![
                workspace(1, "A", true, true),
                workspace(2, "B", true, false),
            ],
            focused_output: Some("A".to_owned()),
        };
        assert_eq!(active_slot(&state, Some("A")), Some(0));
        assert_eq!(active_slot(&state, Some("B")), Some(1));
    }
}
