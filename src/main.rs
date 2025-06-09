use core::runtime::runtime;
use gtk::gdk::Display;
use gtk::{gio, glib};
use gtk::{prelude::*, CssProvider};
use settings::Settings;
use std::sync::LazyLock;
use utils::schema_handler::SchemaHandler;

mod cli;
mod core;
mod paths;
mod settings;
mod ui;
mod utils;

const APP_ID: &str = "com.github.kopecmaciej.fox-reader";

pub static SETTINGS: LazyLock<Settings> = LazyLock::new(Settings::default);

fn main() -> glib::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let is_cli_mode = args.iter().any(|arg| arg == "--cli");

    if args.len() > 1
        && (args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()))
        && !is_cli_mode
    {
        print_general_help();
        return glib::ExitCode::SUCCESS;
    }

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

    if args.iter().any(|arg| arg == "--keybindings" || arg == "-k") {
        print_keybindings();
        return glib::ExitCode::SUCCESS;
    }

    if let Err(e) = runtime().block_on(SchemaHandler::install_from_url()) {
        eprintln!("{}", e);
        return glib::ExitCode::FAILURE;
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

fn print_general_help() {
    println!("Fox Reader - Text-to-Speech Application");
    println!("=======================================\n");
    println!("A modern text-to-speech application with both GUI and CLI modes.\n");

    println!("Usage:");
    println!("  fox-reader                    Start GUI mode (default)");
    println!("  fox-reader --cli <options>    Run in CLI mode");
    println!("  fox-reader -k --keybindings    Show keyboard shortcuts");
    println!("  fox-reader -h --help           Show this help");

    println!("\nCLI Examples:");
    println!("  fox-reader --cli --text \"Hello world\"");
    println!("  fox-reader --cli --list-voices");
    println!("  fox-reader --cli --help");

    println!("\nFor detailed CLI options, run: fox-reader --cli --help");
}

fn print_keybindings() {
    use ui::keybindings::{format_key_combination, KeyBindingManager};

    let manager = KeyBindingManager::new();

    println!("Fox Reader - Keyboard Shortcuts");
    println!("===============================\n");

    for binding in manager.get_all_bindings() {
        let key_combo = format_key_combination(binding.key, binding.modifiers);
        println!("  {:<20} {}", key_combo, binding.description);
    }
}
