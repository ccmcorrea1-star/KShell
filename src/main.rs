mod core;
mod ui;

use std::error::Error;
use std::rc::Rc;

fn main() -> Result<(), Box<dyn Error>> {
    let applications: Rc<[core::desktop::DesktopEntry]> = core::desktop::load_applications().into();

    if let Some(index) = ui::run(Rc::clone(&applications))? {
        let application = &applications[index];
        let mut child = core::launch::launch(application)?;
        if application.terminal {
            child.wait()?;
        }
    }

    Ok(())
}
