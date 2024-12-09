use gtk::{gio, glib};

use gtk::prelude::*;
use gtk::Application;
use ui::window::UI;

mod config;
mod dispatcher;
mod downloader;
mod hf;
mod ui;

const APP_ID: &str = "org.piper-reader";

fn main() -> glib::ExitCode {
    gio::resources_register_include!("compiled.gresource").expect("Failed to register resources.");

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    let ui = UI::new(app);

    ui.setup_ui();
}
