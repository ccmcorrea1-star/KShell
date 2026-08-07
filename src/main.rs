mod desktop;
mod icon;
mod launch;
mod search;
mod ui;

use crossterm::terminal;
use std::error::Error;
use std::fs;

fn main() -> Result<(), Box<dyn Error>> {
    let debug = std::env::args()
        .skip(1)
        .any(|argument| argument == "--debug");
    let applications = desktop::load_applications();

    terminal::enable_raw_mode()?;
    let (picker, picker_diagnostics) = icon::detect_picker();
    terminal::disable_raw_mode()?;

    let result = {
        let mut session = ui::TerminalSession::enter()?;
        let result = ui::run(
            session.terminal_mut(),
            &applications,
            picker,
            picker_diagnostics,
        )?;
        session.leave()?;
        result
    };

    if debug {
        let diagnostics = &result.picker_diagnostics;
        let debug_output = format!(
            "klaucher image detection: protocol={:?} cell_size={}x{} capabilities={:?} query={} TERM={:?} TERM_PROGRAM={:?}",
            diagnostics.protocol,
            diagnostics.cell_size.0,
            diagnostics.cell_size.1,
            diagnostics.capabilities,
            diagnostics.query_result,
            diagnostics.term,
            diagnostics.term_program,
        );
        let _ = fs::write("klaucher-debug.log", debug_output);
    }

    if let Some(index) = result.selected {
        launch::launch(&applications[index])?;
    }

    Ok(())
}
