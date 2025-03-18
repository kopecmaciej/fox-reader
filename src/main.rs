use core::runtime::runtime;

use gtk::gdk::Display;
use gtk::{gio, glib};
use gtk::{prelude::*, CssProvider};

mod cli;
mod core;
mod paths;
mod settings;
mod ui;
mod utils;

const APP_ID: &str = "org.fox-reader";

fn main() -> glib::ExitCode {
    match runtime().block_on(cli::run_cli()) {
        Ok(true) => return glib::ExitCode::SUCCESS,
        Ok(false) => (),
        Err(e) => {
            eprintln!("CLI error: {}", e);
        }
    }

    gio::resources_register_include!("fox-reader.gresource")
        .expect("Failed to register resources.");

    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &adw::Application) {
    let window = ui::window::FoxReaderAppWindow::new(app);
    window.present();
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("../resources/styles/style.css"));

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
