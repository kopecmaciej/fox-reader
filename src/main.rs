use crate::ui::UI;
use gtk::glib;

use gtk::prelude::*;
use gtk::Application;

mod config;
mod downloader;
mod hf;
mod ui;
mod dispatcher;

const APP_ID: &str = "org.piper-reader";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    let ui = UI::new(app);

    ui.setup_ui();
}
