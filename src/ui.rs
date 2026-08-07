use std::io::{self, Stdout};

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use ratatui_image::Image;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    desktop::DesktopEntry,
    icon::{IconCache, PickerDiagnostics, ICON_CELL_SIZE},
    search,
};

const ICON_WIDTH: u16 = ICON_CELL_SIZE.width;
const CONTENT_HEIGHT: u16 = ICON_CELL_SIZE.height;
const ITEM_HEIGHT: u16 = 3;

pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    restored: bool,
}

impl TerminalSession {
    pub fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide) {
            let _ = execute!(stdout, LeaveAlternateScreen, Show);
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
                let _ = execute!(stdout, LeaveAlternateScreen, Show);
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
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
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
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    applications: &[DesktopEntry],
    picker: ratatui_image::picker::Picker,
    picker_diagnostics: PickerDiagnostics,
) -> io::Result<RunResult> {
    let mut query = String::new();
    let mut results = search::filter(applications, &query);
    let mut selected = 0;
    let mut icon_cache = IconCache::new(picker, picker_diagnostics);

    loop {
        terminal.draw(|frame| {
            draw(
                frame,
                applications,
                &query,
                &results,
                selected,
                &mut icon_cache,
            )
        })?;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if key.code == KeyCode::Esc
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    return Ok(RunResult::new(None, icon_cache.diagnostics().clone()));
                }

                match key.code {
                    KeyCode::Enter => {
                        return Ok(RunResult::new(
                            results.get(selected).map(|result| result.index),
                            icon_cache.diagnostics().clone(),
                        ));
                    }
                    KeyCode::Up if !results.is_empty() => {
                        selected = if selected == 0 {
                            results.len() - 1
                        } else {
                            selected - 1
                        };
                    }
                    KeyCode::Down if !results.is_empty() => {
                        selected = (selected + 1) % results.len();
                    }
                    KeyCode::Backspace => {
                        query.pop();
                        results = search::filter(applications, &query);
                        selected = selected.min(results.len().saturating_sub(1));
                    }
                    KeyCode::Char(character)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        query.push(character);
                        results = search::filter(applications, &query);
                        selected = selected.min(results.len().saturating_sub(1));
                    }
                    _ => {}
                }
            }
            Event::Resize(_, _) => icon_cache.on_resize(),
            _ => {}
        }
    }
}

pub struct RunResult {
    pub selected: Option<usize>,
    pub picker_diagnostics: PickerDiagnostics,
}

impl RunResult {
    fn new(selected: Option<usize>, picker_diagnostics: PickerDiagnostics) -> Self {
        Self {
            selected,
            picker_diagnostics,
        }
    }
}

fn draw(
    frame: &mut ratatui::Frame<'_>,
    applications: &[DesktopEntry],
    query: &str,
    results: &[search::SearchResult],
    selected: usize,
    icon_cache: &mut IconCache,
) {
    let [search_area, result_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let search_box = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::LightBlue)),
        Span::raw(query),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(search_box, search_area);

    let list_block = Block::default().borders(Borders::ALL);
    let list_area = list_block.inner(result_area);
    frame.render_widget(list_block, result_area);

    if results.is_empty() {
        frame.render_widget(Paragraph::new("No applications found"), list_area);
    } else if list_area.height >= ITEM_HEIGHT {
        let visible_items = usize::from(list_area.height / ITEM_HEIGHT);
        let max_offset = results.len().saturating_sub(visible_items);
        let offset = selected
            .saturating_sub(visible_items.saturating_sub(1))
            .min(max_offset);
        let item_rects =
            Layout::vertical(vec![Constraint::Length(ITEM_HEIGHT); visible_items]).split(list_area);

        // Paint every item background first. This leaves a clean Ratatui buffer for the
        // following text and image passes when filtering, scrolling, or resizing.
        for visible_index in 0..visible_items {
            let item_rect: Rect = item_rects[visible_index];
            let is_selected = offset + visible_index == selected;
            let item_style = if is_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            frame.render_widget(Block::default().style(item_style), item_rect);
        }

        // Paint the two text lines in the area to the right of the fixed 4-cell icon slot.
        for (visible_index, result) in results.iter().skip(offset).take(visible_items).enumerate() {
            let application = &applications[result.index];
            let item_rect: Rect = item_rects[visible_index];
            let text_rect = text_rect(item_rect);
            let [name_rect, description_rect] =
                Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(text_rect);
            let is_selected = offset + visible_index == selected;
            let item_style = if is_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            let generic_name = application.generic_name.as_deref().unwrap_or_default();
            let name = truncate_to_width(application.name.as_str(), name_rect.width);
            let description = truncate_to_width(generic_name, description_rect.width);

            frame.render_widget(Paragraph::new(name).style(item_style), name_rect);
            frame.render_widget(
                Paragraph::new(description).style(item_style),
                description_rect,
            );
        }

        // Icons are the last pass. Never render a fixed 4x2 protocol into a clipped rectangle.
        if list_area.width >= ICON_WIDTH {
            for (visible_index, result) in
                results.iter().skip(offset).take(visible_items).enumerate()
            {
                let application = &applications[result.index];
                let item_rect: Rect = item_rects[visible_index];
                let icon_rect = Rect::new(item_rect.x, item_rect.y, ICON_WIDTH, CONTENT_HEIGHT);
                if let Some(protocol) = icon_cache.protocol_for(application.icon.as_deref()) {
                    frame.render_widget(Image::new(protocol), icon_rect);
                }
            }
        }
    }

    frame.render_widget(
        Paragraph::new("↑ ↓ navegar    ↵ abrir    esc sair")
            .style(Style::default().fg(Color::DarkGray)),
        footer_area,
    );
}

fn text_rect(item_rect: Rect) -> Rect {
    if item_rect.width < ICON_WIDTH {
        return Rect::new(item_rect.x, item_rect.y, item_rect.width, CONTENT_HEIGHT);
    }

    Rect::new(
        item_rect.x + ICON_WIDTH,
        item_rect.y,
        item_rect.width - ICON_WIDTH,
        CONTENT_HEIGHT,
    )
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
    use ratatui::{backend::TestBackend, layout::Rect, Terminal};

    use super::{draw, text_rect, truncate_to_width, IconCache, PickerDiagnostics};
    use crate::desktop::DesktopEntry;

    fn application(index: usize) -> DesktopEntry {
        DesktopEntry {
            name: format!("Application {index}"),
            generic_name: Some(format!("Description {index}")),
            icon: None,
            exec: vec![format!("application-{index}")],
            working_dir: None,
        }
    }

    fn test_icon_cache() -> IconCache {
        let picker = ratatui_image::picker::Picker::halfblocks();
        let diagnostics = PickerDiagnostics {
            protocol: picker.protocol_type(),
            cell_size: picker.font_size(),
            capabilities: Vec::new(),
            query_result: "test".to_owned(),
            term: None,
            term_program: None,
        };
        IconCache::new(picker, diagnostics)
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
    fn reserves_exactly_four_cells_for_icons() {
        assert_eq!(text_rect(Rect::new(2, 3, 16, 3)), Rect::new(6, 3, 12, 2));
    }

    #[test]
    fn repeated_search_navigation_scroll_and_resize_frames_are_stable() {
        let applications = (0..24).map(application).collect::<Vec<_>>();
        let mut terminal = Terminal::new(TestBackend::new(48, 18)).unwrap();
        let mut icon_cache = test_icon_cache();

        for (query, next_selected, width, height) in [
            ("", 0, 48, 18),
            ("a", 1, 48, 18),
            ("ap", 5, 48, 18),
            ("app", 10, 32, 12),
            ("", 20, 64, 21),
            ("application 2", 0, 64, 21),
            ("", 23, 40, 9),
        ] {
            let results = crate::search::filter(&applications, query);
            let selected = next_selected.min(results.len().saturating_sub(1));
            terminal.backend_mut().resize(width, height);
            icon_cache.on_resize();
            terminal
                .draw(|frame| {
                    draw(
                        frame,
                        &applications,
                        query,
                        &results,
                        selected,
                        &mut icon_cache,
                    )
                })
                .unwrap();
        }
    }
}
