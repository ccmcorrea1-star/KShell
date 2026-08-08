mod desktop;
mod launch;
mod search;
mod ui;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let applications = desktop::load_applications();

    let mut session = ui::TerminalSession::enter()?;
    let result = ui::run(session.terminal_mut(), &applications)?;
    session.leave()?;

    if let Some(index) = result.selected {
        launch::launch(&applications[index])?;
    }

    Ok(())
}
