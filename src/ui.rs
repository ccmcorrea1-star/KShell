use std::io::{self, Stdout};

use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableFocusChange, EnableFocusChange, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
    Terminal,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{desktop::DesktopEntry, search};

const ITEM_HEIGHT: u16 = 1;

pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    restored: bool,
}

impl TerminalSession {
    pub fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnableFocusChange, EnterAlternateScreen, Hide) {
            let _ = execute!(stdout, DisableFocusChange, LeaveAlternateScreen, Show);
            let _ = terminal::disable_raw_mode();
            return Err(error);
        }

        match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => Ok(Self {
                terminal,
                restored: false,
            }),
            Err(error) => {
                let mut stdout = io::stdout();
                let _ = execute!(stdout, DisableFocusChange, LeaveAlternateScreen, Show);
                let _ = terminal::disable_raw_mode();
                Err(error)
            }
        }
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    pub fn leave(mut self) -> io::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            DisableFocusChange,
            LeaveAlternateScreen
        )?;
        self.terminal.show_cursor()?;
        self.restored = true;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.restored {
            return;
        }

        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            DisableFocusChange,
            LeaveAlternateScreen
        );
        let _ = self.terminal.show_cursor();
    }
}

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    applications: &[DesktopEntry],
) -> io::Result<RunResult> {
    let mut query = String::new();
    let mut results = search::filter(applications, &query);
    let mut selected = 0;

    loop {
        terminal.draw(|frame| draw(frame, applications, &query, &results, selected))?;

        match event::read()? {
            Event::Key(key) => {
                if let Some(result) =
                    handle_key_event(key, applications, &mut query, &mut results, &mut selected)
                {
                    return Ok(result);
                }
            }
            Event::FocusLost => return Ok(RunResult::new(None)),
            _ => {}
        }
    }
}

fn handle_key_event(
    key: KeyEvent,
    applications: &[DesktopEntry],
    query: &mut String,
    results: &mut Vec<search::SearchResult>,
    selected: &mut usize,
) -> Option<RunResult> {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }

    if key.code == KeyCode::Esc
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
    {
        return Some(RunResult::new(None));
    }

    match key.code {
        KeyCode::Enter => Some(RunResult::new(
            results.get(*selected).map(|result| result.index),
        )),
        KeyCode::Up if !results.is_empty() => {
            *selected = if *selected == 0 {
                results.len() - 1
            } else {
                *selected - 1
            };
            None
        }
        KeyCode::Down if !results.is_empty() => {
            *selected = (*selected + 1) % results.len();
            None
        }
        KeyCode::Backspace => {
            remove_last_grapheme(query);
            *results = search::filter(applications, query);
            *selected = (*selected).min(results.len().saturating_sub(1));
            None
        }
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            query.push(character);
            *results = search::filter(applications, query);
            *selected = (*selected).min(results.len().saturating_sub(1));
            None
        }
        _ => None,
    }
}

fn remove_last_grapheme(value: &mut String) {
    if let Some((index, _)) = value.grapheme_indices(true).next_back() {
        value.truncate(index);
    }
}

pub struct RunResult {
    pub selected: Option<usize>,
}

impl RunResult {
    fn new(selected: Option<usize>) -> Self {
        Self { selected }
    }
}

#[derive(Clone, Debug)]
struct ItemLayout {
    application_index: usize,
    selected: bool,
}

#[derive(Clone, Debug)]
struct UiLayout {
    search_area: Rect,
    result_area: Rect,
    footer_area: Rect,
    list_content_area: Rect,
    scrollbar_area: Option<Rect>,
    offset: usize,
    visible_items: usize,
    items: Vec<ItemLayout>,
}

fn calculate_layout(area: Rect, results: &[search::SearchResult], selected: usize) -> UiLayout {
    let [search_area, result_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    let list_block = Block::default().padding(Padding::horizontal(1));
    let list_area = list_block.inner(result_area);
    let mut items = Vec::new();
    let mut offset = 0;
    let mut visible_items = 0;

    if !results.is_empty() && list_area.height >= ITEM_HEIGHT {
        visible_items = usize::from(list_area.height / ITEM_HEIGHT);
        let max_offset = results.len().saturating_sub(visible_items);
        offset = selected
            .saturating_sub(visible_items.saturating_sub(1))
            .min(max_offset);

        for (visible_index, result) in results.iter().skip(offset).take(visible_items).enumerate() {
            items.push(ItemLayout {
                application_index: result.index,
                selected: offset + visible_index == selected,
            });
        }
    }

    let scrollbar_area = (results.len() > visible_items && list_area.width >= 2).then(|| {
        Rect::new(
            list_area.right().saturating_sub(1),
            list_area.y,
            1,
            list_area.height,
        )
    });
    let list_content_area = scrollbar_area
        .map(|scrollbar_area| {
            Rect::new(
                list_area.x,
                list_area.y,
                list_area.width.saturating_sub(scrollbar_area.width),
                list_area.height,
            )
        })
        .unwrap_or(list_area);

    UiLayout {
        search_area,
        result_area,
        footer_area,
        list_content_area,
        scrollbar_area,
        offset,
        visible_items,
        items,
    }
}

fn draw(
    frame: &mut ratatui::Frame<'_>,
    applications: &[DesktopEntry],
    query: &str,
    results: &[search::SearchResult],
    selected: usize,
) {
    let layout = calculate_layout(frame.area(), results, selected);

    let search_box = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::LightBlue)),
        Span::styled(query, Style::default().fg(Color::White)),
    ]))
    .style(Style::default().fg(Color::Gray))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray))
            .padding(Padding::new(1, 1, 1, 0)),
    );
    frame.render_widget(search_box, layout.search_area);

    frame.render_widget(
        Block::default().padding(Padding::horizontal(1)),
        layout.result_area,
    );

    if results.is_empty() {
        frame.render_widget(
            Paragraph::new("No applications found").style(Style::default().fg(Color::DarkGray)),
            layout.list_content_area,
        );
    } else {
        let name_width = layout.list_content_area.width.saturating_sub(2);
        let list_items = layout
            .items
            .iter()
            .map(|item| {
                let application = &applications[item.application_index];
                ListItem::new(truncate_to_width(application.name.as_str(), name_width))
            })
            .collect::<Vec<_>>();
        let selected_item = layout.items.iter().position(|item| item.selected);
        let mut list_state = ListState::default();
        list_state.select(selected_item);

        let list = List::new(list_items)
            .style(Style::default().fg(Color::Gray))
            .highlight_style(
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_symbol("› ");
        frame.render_stateful_widget(list, layout.list_content_area, &mut list_state);

        if let Some(scrollbar_area) = layout.scrollbar_area {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_symbol("▐")
                .thumb_style(Style::default().fg(Color::DarkGray))
                .track_symbol(None)
                .begin_symbol(None)
                .end_symbol(None);
            let mut scrollbar_state = ScrollbarState::new(results.len())
                .position(layout.offset)
                .viewport_content_length(layout.visible_items);
            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }
    }

    frame.render_widget(
        Paragraph::new("↑ ↓ navegar    ↵ abrir    esc sair")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().padding(Padding::horizontal(1))),
        layout.footer_area,
    );
}

fn truncate_to_width(value: &str, width: u16) -> String {
    let width = usize::from(width);
    let normalized = value
        .graphemes(true)
        .map(|grapheme| {
            if grapheme.chars().any(char::is_control) {
                " "
            } else {
                grapheme
            }
        })
        .collect::<String>();

    if Line::from(normalized.as_str()).width() <= width {
        return normalized;
    }

    if width == 0 {
        return String::new();
    }

    let ellipsis = "…";
    let ellipsis_width = Line::from(ellipsis).width();
    let content_width = width.saturating_sub(ellipsis_width);
    let mut truncated = String::new();

    for grapheme in normalized.graphemes(true) {
        let mut candidate = truncated.clone();
        candidate.push_str(grapheme);
        if Line::from(candidate.as_str()).width() > content_width {
            break;
        }
        truncated.push_str(grapheme);
    }

    truncated.push_str(ellipsis);
    truncated
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};

    use super::{draw, handle_key_event, remove_last_grapheme, truncate_to_width};
    use crate::desktop::DesktopEntry;

    fn application(index: usize) -> DesktopEntry {
        DesktopEntry {
            name: format!("Application {index}"),
            generic_name: Some(format!("Description {index}")),
            exec: vec![format!("application-{index}")],
            working_dir: None,
            terminal: false,
        }
    }

    fn apply_key(
        kind: KeyEventKind,
        code: KeyCode,
        applications: &[DesktopEntry],
        query: &mut String,
        results: &mut Vec<crate::search::SearchResult>,
        selected: &mut usize,
    ) {
        assert!(handle_key_event(
            KeyEvent::new_with_kind(code, KeyModifiers::empty(), kind),
            applications,
            query,
            results,
            selected,
        )
        .is_none());
    }

    #[test]
    fn removes_complete_unicode_graphemes() {
        let mut value = "a👩‍💻e\u{301}".to_owned();

        remove_last_grapheme(&mut value);
        assert_eq!(value, "a👩‍💻");

        remove_last_grapheme(&mut value);
        assert_eq!(value, "a");
    }

    #[test]
    fn repeats_match_pressed_keys_for_query_and_navigation() {
        let applications = (0..3).map(application).collect::<Vec<_>>();
        let mut press_query = String::new();
        let mut repeat_query = String::new();
        let mut press_results = crate::search::filter(&applications, &press_query);
        let mut repeat_results = crate::search::filter(&applications, &repeat_query);
        let mut press_selected = 0;
        let mut repeat_selected = 0;

        apply_key(
            KeyEventKind::Press,
            KeyCode::Char('a'),
            &applications,
            &mut press_query,
            &mut press_results,
            &mut press_selected,
        );
        apply_key(
            KeyEventKind::Repeat,
            KeyCode::Char('a'),
            &applications,
            &mut repeat_query,
            &mut repeat_results,
            &mut repeat_selected,
        );
        assert_eq!(press_query, repeat_query);
        assert_eq!(press_results, repeat_results);

        apply_key(
            KeyEventKind::Press,
            KeyCode::Up,
            &applications,
            &mut press_query,
            &mut press_results,
            &mut press_selected,
        );
        apply_key(
            KeyEventKind::Repeat,
            KeyCode::Up,
            &applications,
            &mut repeat_query,
            &mut repeat_results,
            &mut repeat_selected,
        );
        assert_eq!(press_selected, repeat_selected);

        apply_key(
            KeyEventKind::Press,
            KeyCode::Backspace,
            &applications,
            &mut press_query,
            &mut press_results,
            &mut press_selected,
        );
        apply_key(
            KeyEventKind::Repeat,
            KeyCode::Backspace,
            &applications,
            &mut repeat_query,
            &mut repeat_results,
            &mut repeat_selected,
        );
        assert_eq!(press_query, repeat_query);
        assert_eq!(press_results, repeat_results);
        assert_eq!(press_selected, repeat_selected);
    }

    #[test]
    fn truncates_to_terminal_width_without_wrapping() {
        assert_eq!(
            truncate_to_width("Equalizer, Compressor", 12),
            "Equalizer, …"
        );
    }

    #[test]
    fn measures_wide_unicode_display_cells() {
        assert_eq!(truncate_to_width("日本語の設定", 5), "日本…");
    }

    #[test]
    fn normalizes_control_characters_to_one_line() {
        assert_eq!(
            truncate_to_width("Name\nDescription", 20),
            "Name Description"
        );
    }

    #[test]
    fn names_render_without_image_dependencies() {
        let applications = vec![application(0), application(1)];
        let results = crate::search::filter(&applications, "");
        let mut terminal = Terminal::new(TestBackend::new(48, 8)).unwrap();

        terminal
            .draw(|frame| draw(frame, &applications, "", &results, 0))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("Application 0"));
        assert!(rendered.contains("Application 1"));
        assert!(!rendered.contains("Description"));
    }

    #[test]
    fn scrollbar_is_rendered_only_when_results_overflow() {
        let applications = (0..10).map(application).collect::<Vec<_>>();
        let results = crate::search::filter(&applications, "");
        let mut terminal = Terminal::new(TestBackend::new(48, 8)).unwrap();

        terminal
            .draw(|frame| draw(frame, &applications, "", &results, 0))
            .unwrap();
        let overflow_rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.symbol() == "▐");
        assert!(overflow_rendered);

        terminal.backend_mut().resize(48, 20);
        terminal
            .draw(|frame| draw(frame, &applications, "", &results, 0))
            .unwrap();
        let fits_rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.symbol() == "▐");
        assert!(!fits_rendered);
    }

    #[test]
    fn repeated_search_navigation_and_resize_frames_are_stable() {
        let applications = (0..24).map(application).collect::<Vec<_>>();
        let mut terminal = Terminal::new(TestBackend::new(48, 18)).unwrap();

        for (query, next_selected, width, height) in [
            ("", 0, 48, 18),
            ("a", 1, 48, 18),
            ("ap", 5, 48, 12),
            ("app", 10, 32, 8),
            ("", 20, 64, 21),
            ("application 2", 0, 64, 21),
            ("", 23, 40, 6),
        ] {
            let results = crate::search::filter(&applications, query);
            let selected = next_selected.min(results.len().saturating_sub(1));
            terminal.backend_mut().resize(width, height);
            terminal
                .draw(|frame| draw(frame, &applications, query, &results, selected))
                .unwrap();
        }
    }
}
