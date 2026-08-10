mod calendar;
mod clock;
mod niri;
mod system;
mod ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ui::run()
}
