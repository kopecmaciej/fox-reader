use crate::ui::UI;
use gtk::glib;

mod config;
mod downloader;
mod hf;
mod ui;

fn main() -> glib::ExitCode {
    let ui = UI::new();
    ui.run()
}

