mod gtk;
mod output;
mod selection;

pub(crate) use output::OutputContext;

use std::error::Error;
use std::rc::Rc;

use crate::core::desktop::DesktopEntry;

pub fn run(applications: Rc<[DesktopEntry]>) -> Result<Option<usize>, Box<dyn Error>> {
    gtk::run(applications)
}
