use adw::ColorScheme;
use gtk::gdk::RGBA;
use gtk::pango::FontDescription;
use serde::{Deserialize, Serialize};

use crate::{core::file_handler::FileHandler, paths::get_app_config_path};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserConfig {
    pub font: Option<String>,
    pub highlight_color: Option<String>,
    pub theme: Option<String>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            font: None,
            highlight_color: None,
            theme: Some("system".to_string()),
        }
    }
}

impl UserConfig {
    pub fn new() -> Self {
        FileHandler::load_settings_from_json(&Self::get_config_path()).unwrap_or_default()
    }

    pub fn get_config_path() -> String {
        get_app_config_path()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        FileHandler::update_json(&Self::get_config_path(), self)
    }

    pub fn get_font_description(&self) -> Option<FontDescription> {
        self.font
            .as_ref()
            .map(|font_str| FontDescription::from_string(font_str))
    }

    pub fn get_highlight_rgba(&self) -> RGBA {
        let initial_rgba = gtk::gdk::RGBA::new(1.0, 1.0, 0.0, 0.3);
        self.highlight_color
            .as_ref()
            .and_then(|color_str| RGBA::parse(color_str).ok())
            .unwrap_or(initial_rgba)
    }

    pub fn get_color_scheme(&self) -> ColorScheme {
        match self.theme.as_deref() {
            Some("light") => ColorScheme::ForceLight,
            Some("dark") => ColorScheme::ForceDark,
            _ => ColorScheme::Default,
        }
    }

    pub fn set_font(&mut self, font_desc: &FontDescription) {
        self.font = Some(font_desc.to_string());
        if let Err(e) = self.save() {
            eprintln!("Failed to save font settings: {}", e);
        }
    }

    pub fn set_highlight_color(&mut self, rgba: &RGBA) {
        self.highlight_color = Some(rgba.to_string());
        if let Err(e) = self.save() {
            eprintln!("Failed to save highlight color settings: {}", e);
        }
    }

    pub fn set_theme(&mut self, is_dark: bool) {
        self.theme = Some(if is_dark { "dark" } else { "light" }.to_string());
        if let Err(e) = self.save() {
            eprintln!("Failed to save theme settings: {}", e);
        }
    }
}
