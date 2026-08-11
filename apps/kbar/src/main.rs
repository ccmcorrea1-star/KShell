mod app;
mod calendar;
mod clock;
mod services;
mod ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
