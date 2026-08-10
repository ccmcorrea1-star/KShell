mod gtk;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    gtk::run()
}
