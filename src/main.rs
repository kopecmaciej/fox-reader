use crate::hf::HuggingFace;
use crate::ui::UI;
use gtk::glib;

mod config;
mod downloader;
mod hf;
mod ui;

fn main() -> glib::ExitCode {
    let hf = HuggingFace::new();
    match hf.parse_avaliable_voices() {
        Ok(v) => println!("{:?}", v),
        Err(e) => panic!("{e}"),
    }

    let ui = UI::new();
    ui.run()
}
