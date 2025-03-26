use core::runtime::runtime;
use gtk::gdk::Display;
use gtk::{gio, glib};
use gtk::{prelude::*, CssProvider};
use settings::Settings;
use std::sync::LazyLock;

mod cli;
mod core;
mod paths;
mod settings;
mod ui;
mod utils;

const APP_ID: &str = "org.fox-reader";

pub static SETTINGS: LazyLock<Settings> = LazyLock::new(Settings::default);

fn main() -> glib::ExitCode {
    let is_cli_mode = std::env::args().any(|arg| &arg == "--cli");

    if is_cli_mode {
        match runtime().block_on(cli::run_cli()) {
            Ok(true) => return glib::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}", e);
                return glib::ExitCode::FAILURE;
            }
            _ => {
                return glib::ExitCode::FAILURE;
            }
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
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("../resources/styles/style.css"));

    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
