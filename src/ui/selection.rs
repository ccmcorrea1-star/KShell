#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SelectionState {
    results: Vec<usize>,
    selected_row: Option<usize>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_results(&mut self, results: Vec<usize>) {
        self.results = results;
        self.selected_row = (!self.results.is_empty()).then_some(0);
    }

    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    pub fn selected_row(&self) -> Option<usize> {
        self.selected_row
    }

    pub fn navigate(&mut self, direction: Direction) -> Option<usize> {
        let count = self.results.len();
        if count == 0 {
            return None;
        }

        let row = match self.selected_row {
            None => 0,
            Some(row) if row >= count => 0,
            Some(row) => match direction {
                Direction::Up => row.checked_sub(1).unwrap_or(count - 1),
                Direction::Down => (row + 1) % count,
            },
        };
        self.selected_row = Some(row);
        self.selected_row
    }

    pub fn application_index_at(&self, row: usize) -> Option<usize> {
        self.results.get(row).copied()
    }

    pub fn activate_row(&self, row: usize) -> Option<usize> {
        self.application_index_at(row)
    }

    pub fn activate_selected(&self) -> Option<usize> {
        self.selected_row
            .and_then(|row| self.application_index_at(row))
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, SelectionState};

    #[test]
    fn non_empty_results_select_the_first_ordered_result() {
        let mut state = SelectionState::new();

        state.update_results(vec![7, 3]);

        assert_eq!(state.selected_row(), Some(0));
        assert_eq!(state.activate_selected(), Some(7));
        assert_eq!(state.result_count(), 2);
    }

    #[test]
    fn replacing_non_empty_results_resets_selection_to_row_zero() {
        let mut state = SelectionState::new();
        state.update_results(vec![7, 3]);
        state.navigate(Direction::Down);

        state.update_results(vec![9, 5]);

        assert_eq!(state.selected_row(), Some(0));
        assert_eq!(state.activate_selected(), Some(9));
        assert_eq!(state.result_count(), 2);
        assert_eq!(state.application_index_at(1), Some(5));
    }

    #[test]
    fn empty_results_clear_selection() {
        let mut state = SelectionState::new();
        state.update_results(vec![7, 3]);

        state.update_results(Vec::new());

        assert_eq!(state.selected_row(), None);
        assert_eq!(state.activate_selected(), None);
        assert_eq!(state.result_count(), 0);
    }

    #[test]
    fn navigation_wraps_in_both_directions() {
        let mut state = SelectionState::new();
        state.update_results(vec![7, 3, 9]);

        assert_eq!(state.navigate(Direction::Up), Some(2));
        assert_eq!(state.navigate(Direction::Down), Some(0));
        assert_eq!(state.navigate(Direction::Down), Some(1));
        assert_eq!(state.navigate(Direction::Up), Some(0));
    }

    #[test]
    fn activating_an_invalid_row_returns_none_and_preserves_selection() {
        let mut state = SelectionState::new();
        state.update_results(vec![7, 3]);

        assert_eq!(state.activate_row(2), None);
        assert_eq!(state.selected_row(), Some(0));
    }

    #[test]
    fn activation_without_selection_returns_none() {
        let mut state = SelectionState::new();
        state.update_results(Vec::new());

        assert_eq!(state.activate_selected(), None);

        state.update_results(vec![7, 3]);

        assert_eq!(state.activate_row(0), Some(7));
        assert_eq!(state.activate_row(2), None);
    }

    #[test]
    fn navigation_without_results_and_activation_of_empty_results_return_none() {
        let mut state = SelectionState::new();
        state.update_results(Vec::new());

        assert_eq!(state.navigate(Direction::Up), None);
        assert_eq!(state.navigate(Direction::Down), None);
        assert_eq!(state.selected_row(), None);
        assert_eq!(state.activate_selected(), None);
        assert_eq!(state.activate_row(0), None);
    }
}
