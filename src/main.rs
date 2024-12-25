use gtk::prelude::*;
use gtk::{gio, glib};

mod config;
mod core;
mod ui;

const APP_ID: &str = "org.fox-reader";

fn main() -> glib::ExitCode {
    gio::resources_register_include!("fox-reader.gresource")
        .expect("Failed to register resources.");

    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &adw::Application) {
    let window = ui::window::FoxReaderAppWindow::new(app);
    window.present();
}
