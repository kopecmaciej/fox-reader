use crate::hf::HuggingFace;
use crate::ui::UI;
use gtk::prelude::*;
use gtk::{glib, Application};

mod config;
mod downloader;
mod hf;
mod ui;

const APP_ID: &str = "piper-reader";

fn main() -> glib::ExitCode {
    let hf = HuggingFace::new();
    match hf.parse_avaliable_voices() {
        Ok(v) => println!("{:?}", v),
        Err(e) => panic!("{e}"),
    }

    let app = Application::builder().application_id(APP_ID).build();
    let ui = UI::new();

    app.connect_activate(|app| {
        ui.build_ui(app);
    });

    app.run()
}
