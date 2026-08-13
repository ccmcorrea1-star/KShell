//! Shared keyboard-interaction lifecycle for bar popovers.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gtk4 as gtk;
use gtk4_layer_shell::{KeyboardMode, LayerShell};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PopoverId {
    Calendar,
    Volume,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PopoverState {
    active: Option<PopoverId>,
}

type CloseHandler = Rc<dyn Fn()>;
type CloseHandlers = Rc<RefCell<HashMap<PopoverId, CloseHandler>>>;

impl PopoverState {
    pub fn open(&mut self, id: PopoverId) -> Option<PopoverId> {
        let previous = self.active;
        self.active = Some(id);
        previous
    }

    pub fn close(&mut self, id: PopoverId) -> bool {
        if self.active != Some(id) {
            return false;
        }
        self.active = None;
        true
    }

    pub fn is_active(&self, id: PopoverId) -> bool {
        self.active == Some(id)
    }

    #[cfg(test)]
    pub fn active(&self) -> Option<PopoverId> {
        self.active
    }
}

#[derive(Clone)]
pub struct PopoverCoordinator {
    state: Rc<Cell<PopoverState>>,
    window: gtk::ApplicationWindow,
    close_handlers: CloseHandlers,
}

impl PopoverCoordinator {
    pub fn new(window: &gtk::ApplicationWindow) -> Self {
        Self {
            state: Rc::new(Cell::new(PopoverState::default())),
            window: window.clone(),
            close_handlers: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn register<F>(&self, id: PopoverId, close: F)
    where
        F: Fn() + 'static,
    {
        self.close_handlers.borrow_mut().insert(id, Rc::new(close));
    }

    pub fn open(&self, id: PopoverId) {
        let previous = self.state.get().open(id);
        self.state.set(PopoverState { active: Some(id) });
        if let Some(previous) = previous.filter(|previous| *previous != id) {
            if let Some(close) = self.close_handlers.borrow().get(&previous).cloned() {
                close();
            }
        }
        if id == PopoverId::Calendar {
            self.window.set_keyboard_mode(KeyboardMode::OnDemand);
        } else if previous == Some(PopoverId::Calendar) {
            self.window.set_keyboard_mode(KeyboardMode::None);
        }
    }

    pub fn is_active(&self, id: PopoverId) -> bool {
        self.state.get().is_active(id)
    }

    pub fn close(&self, id: PopoverId) {
        let mut state = self.state.get();
        if !state.close(id) {
            return;
        }
        self.state.set(state);
        if id == PopoverId::Calendar {
            self.window.set_keyboard_mode(KeyboardMode::None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PopoverId, PopoverState};

    #[test]
    fn an_old_close_cannot_clear_a_newer_popover() {
        let mut state = PopoverState::default();
        assert_eq!(state.open(PopoverId::Volume), None);
        assert_eq!(state.open(PopoverId::Calendar), Some(PopoverId::Volume));
        assert!(!state.close(PopoverId::Volume));
        assert_eq!(state.active(), Some(PopoverId::Calendar));
        assert!(state.close(PopoverId::Calendar));
        assert_eq!(state.active(), None);
    }

    #[test]
    fn the_close_guard_is_symmetric_when_switching_back_to_volume() {
        let mut state = PopoverState::default();
        assert_eq!(state.open(PopoverId::Calendar), None);
        assert_eq!(state.open(PopoverId::Volume), Some(PopoverId::Calendar));
        assert!(!state.close(PopoverId::Calendar));
        assert_eq!(state.active(), Some(PopoverId::Volume));
        assert!(state.close(PopoverId::Volume));
        assert_eq!(state.active(), None);
    }

    #[test]
    fn active_owner_can_be_checked_before_toggling() {
        let mut state = PopoverState::default();
        assert!(!state.is_active(PopoverId::Volume));
        state.open(PopoverId::Volume);
        assert!(state.is_active(PopoverId::Volume));
        assert!(!state.is_active(PopoverId::Calendar));
    }
}
