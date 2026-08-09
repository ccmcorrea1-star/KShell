mod gtk;
mod selection;

use std::error::Error;
use std::rc::Rc;

use crate::core::desktop::DesktopEntry;

pub fn run(applications: Rc<[DesktopEntry]>) -> Result<Option<usize>, Box<dyn Error>> {
    gtk::run(applications)
}
