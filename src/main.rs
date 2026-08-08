mod desktop;
mod launch;
mod search;
mod ui;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let applications = desktop::load_applications();

    let result = {
        let _signals = ui::SignalGuard::install()?;
        let mut session = ui::TerminalSession::enter()?;
        let result = ui::run(session.terminal_mut(), &applications)?;
        session.leave()?;
        result
    };

    if ui::termination_requested() {
        return Ok(());
    }

    if let Some(index) = result.selected {
        let application = &applications[index];
        let mut child = launch::launch(application)?;
        if application.terminal {
            child.wait()?;
        }
    }

    Ok(())
}
