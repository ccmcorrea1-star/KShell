mod gtk;
#[allow(dead_code)]
pub mod tui;

use std::error::Error;
use std::rc::Rc;

use crate::core::desktop::DesktopEntry;

pub fn run(applications: Rc<[DesktopEntry]>) -> Result<Option<usize>, Box<dyn Error>> {
    gtk::run(applications)
}
