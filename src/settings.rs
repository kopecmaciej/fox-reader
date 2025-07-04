use adw::ColorScheme;
use gio::prelude::SettingsExt;
use gtk::{gdk::RGBA, gio, glib, pango::FontDescription};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, ops::Deref};

use crate::{paths::whisper_config::get_whisper_models_names, APP_ID};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LLMProvider {
    LMStudio,
    Ollama,
    OpenAI,
    Anthropic,
}

impl LLMProvider {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "LM Studio" => Some(LLMProvider::LMStudio),
            "Ollama" => Some(LLMProvider::Ollama),
            "OpenAI" => Some(LLMProvider::OpenAI),
            "Anthropic" => Some(LLMProvider::Anthropic),
            _ => None,
        }
    }

    pub fn get_all() -> Vec<LLMProvider> {
        vec![
            LLMProvider::LMStudio,
            LLMProvider::Ollama,
            LLMProvider::OpenAI,
            LLMProvider::Anthropic,
        ]
    }

    pub fn get_all_str() -> Vec<String> {
        LLMProvider::get_all()
            .into_iter()
            .map(|p| p.to_string())
            .collect()
    }
}

impl Display for LLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMProvider::LMStudio => write!(f, "LM Studio"),
            LLMProvider::Ollama => write!(f, "Ollama"),
            LLMProvider::OpenAI => write!(f, "OpenAI"),
            LLMProvider::Anthropic => write!(f, "Anthropic"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Settings(gio::Settings);

impl Settings {
    pub fn get_window_width(&self) -> i32 {
        self.int("window-width")
    }

    pub fn set_window_width(&self, width: i32) {
        self.set_int("window-width", width)
            .expect("Failed to set window width");
    }

    pub fn get_window_height(&self) -> i32 {
        self.int("window-height")
    }

    pub fn set_window_height(&self, height: i32) {
        self.set_int("window-height", height)
            .expect("Failed to set window height");
    }

    pub fn get_window_maximized(&self) -> bool {
        self.boolean("window-maximized")
    }

    pub fn set_window_maximized(&self, maximized: bool) {
        self.set_boolean("window-maximized", maximized)
            .expect("Failed to set window maximized state");
    }

    pub fn get_font_description(&self) -> FontDescription {
        let font_str = self.string("font");
        FontDescription::from_string(&font_str)
    }

    pub fn set_font(&self, font_desc: &FontDescription) {
        self.set_string("font", &font_desc.to_string())
            .expect("Failed to set font setting");
    }

    pub fn get_highlight_rgba(&self) -> RGBA {
        let color_str = self.string("highlight-color");
        RGBA::parse(&color_str).unwrap_or_else(|_| RGBA::new(1.0, 1.0, 0.0, 0.3))
    }

    pub fn set_highlight_color(&self, rgba: &gtk::gdk::RGBA) {
        self.set_string("highlight-color", &rgba.to_string())
            .expect("Failed to set highlight color setting");
    }

    pub fn get_color_scheme(&self) -> ColorScheme {
        match self.string("theme").as_str() {
            "light" => ColorScheme::ForceLight,
            "dark" => ColorScheme::ForceDark,
            _ => ColorScheme::Default,
        }
    }

    pub fn is_dark_color_scheme(&self) -> bool {
        self.get_color_scheme() == ColorScheme::ForceDark
    }

    pub fn set_theme(&self, is_dark: bool) {
        let theme = if is_dark { "dark" } else { "light" };
        self.set_string("theme", theme)
            .expect("Failed to set theme setting");
    }

    pub fn get_active_provider_index(&self) -> usize {
        LLMProvider::get_all()
            .iter()
            .position(|p| *p == self.get_active_provider())
            .unwrap_or(0)
    }

    pub fn get_active_provider_str(&self) -> String {
        self.string("active-provider").to_string()
    }

    pub fn get_active_provider(&self) -> LLMProvider {
        let active_provider = self.get_active_provider_str();
        LLMProvider::from_str(&active_provider).unwrap_or(LLMProvider::Ollama)
    }

    pub fn set_active_provider(&self, provider: &str) {
        self.set_string("active-provider", provider)
            .expect("Failed to set active provider");
    }

    pub fn get_active_provider_config(&self) -> ProviderConfig {
        ProviderConfig {
            api_key: Some(self.get_api_key()),
            base_url: self.get_base_url(),
            model: Some(self.get_model()),
            temperature: Some(self.get_temperature() as f32),
            max_tokens: Some(self.get_max_tokens()),
        }
    }

    pub fn get_api_key(&self) -> String {
        let active_provider = self.get_active_provider();
        let key = match active_provider {
            LLMProvider::LMStudio => "lmstudio-api-key",
            LLMProvider::Ollama => "ollama-api-key",
            LLMProvider::OpenAI => "openai-api-key",
            LLMProvider::Anthropic => "anthropic-api-key",
        };

        self.string(key).to_string()
    }

    pub fn set_api_key(&self, api_key: &str) {
        let key = match self.get_active_provider() {
            LLMProvider::LMStudio => "lmstudio-api-key",
            LLMProvider::Ollama => "ollama-api-key",
            LLMProvider::OpenAI => "openai-api-key",
            LLMProvider::Anthropic => "anthropic-api-key",
        };
        self.set_string(key, api_key)
            .expect("Failed to set API key");
    }

    pub fn get_base_url(&self) -> String {
        let key = match self.get_active_provider() {
            LLMProvider::LMStudio => "lmstudio-base-url",
            LLMProvider::Ollama => "ollama-base-url",
            LLMProvider::OpenAI => "openai-base-url",
            LLMProvider::Anthropic => "anthropic-base-url",
        };
        self.string(key).to_string()
    }

    pub fn set_base_url(&self, url: &str) {
        let key = match self.get_active_provider() {
            LLMProvider::LMStudio => "lmstudio-base-url",
            LLMProvider::Ollama => "ollama-base-url",
            LLMProvider::OpenAI => "openai-base-url",
            LLMProvider::Anthropic => "anthropic-base-url",
        };
        self.set_string(key, url).expect("Failed to set base URL");
    }

    pub fn get_model(&self) -> String {
        let key = match self.get_active_provider() {
            LLMProvider::LMStudio => "lmstudio-model",
            LLMProvider::Ollama => "ollama-model",
            LLMProvider::OpenAI => "openai-model",
            LLMProvider::Anthropic => "anthropic-model",
        };
        self.string(key).to_string()
    }

    pub fn set_model(&self, model: &str) {
        let active_provider = self.get_active_provider();
        let key = match active_provider {
            LLMProvider::LMStudio => "lmstudio-model",
            LLMProvider::Ollama => "ollama-model",
            LLMProvider::OpenAI => "openai-model",
            LLMProvider::Anthropic => "anthropic-model",
        };
        self.set_string(key, model).expect("Failed to set model");
    }

    pub fn get_temperature(&self) -> f64 {
        let key = match self.get_active_provider() {
            LLMProvider::LMStudio => "lmstudio-temperature",
            LLMProvider::Ollama => "ollama-temperature",
            LLMProvider::OpenAI => "openai-temperature",
            LLMProvider::Anthropic => "anthropic-temperature",
        };
        self.double(key)
    }

    pub fn set_temperature(&self, temp: f64) {
        let key = match self.get_active_provider() {
            LLMProvider::LMStudio => "lmstudio-temperature",
            LLMProvider::Ollama => "ollama-temperature",
            LLMProvider::OpenAI => "openai-temperature",
            LLMProvider::Anthropic => "anthropic-temperature",
        };
        self.set_double(key, temp)
            .expect("Failed to set temperature");
    }

    pub fn get_max_tokens(&self) -> u32 {
        let key = match self.get_active_provider() {
            LLMProvider::LMStudio => "lmstudio-max-tokens",
            LLMProvider::Ollama => "ollama-max-tokens",
            LLMProvider::OpenAI => "openai-max-tokens",
            LLMProvider::Anthropic => "anthropic-max-tokens",
        };
        self.uint(key)
    }

    pub fn set_max_tokens(&self, max_tokens: u32) {
        let key = match self.get_active_provider() {
            LLMProvider::LMStudio => "lmstudio-max-tokens",
            LLMProvider::Ollama => "ollama-max-tokens",
            LLMProvider::OpenAI => "openai-max-tokens",
            LLMProvider::Anthropic => "anthropic-max-tokens",
        };
        self.set_uint(key, max_tokens)
            .expect("Failed to set max tokens");
    }

    pub fn connect_theme_changed<F: Fn(&gio::Settings, &str) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_changed(Some("theme"), move |s, key| {
            f(s, key);
        })
    }

    pub fn connect_font_changed<F: Fn(&gio::Settings, &str) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_changed(Some("font"), move |s, key| {
            f(s, key);
        })
    }

    pub fn connect_highlight_color_changed<F: Fn(&gio::Settings, &str) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_changed(Some("highlight-color"), move |s, key| {
            f(s, key);
        })
    }

    pub fn connect_active_provider_changed<F: Fn(&gio::Settings, &str) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_changed(Some("active-provider"), move |s, key| {
            f(s, key);
        })
    }

    // Whisper
    pub fn get_whisper_model(&self) -> String {
        self.string("whisper-model").to_string()
    }

    pub fn set_whisper_model(&self, model: &str) {
        self.set_string("whisper-model", model)
            .expect("Failed to set Whisper model");
    }

    pub fn get_active_model_index(&self) -> usize {
        get_whisper_models_names()
            .iter()
            .position(|p| p == &self.get_whisper_model())
            .unwrap_or(0)
    }

    pub fn connect_whisper_model_changed<F: Fn(&gio::Settings, &str) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_changed(Some("whisper-model"), move |s, key| {
            f(s, key);
        })
    }

    pub fn connect_whisper_models_path_changed<F: Fn(&gio::Settings, &str) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_changed(Some("whisper-models-path"), move |s, key| {
            f(s, key);
        })
    }

    // Voice Settings
    pub fn get_default_voice(&self) -> String {
        self.string("default-voice").to_string()
    }

    pub fn set_default_voice(&self, voice_key: &str) {
        self.set_string("default-voice", voice_key)
            .expect("Failed to set default voice");
    }

    pub fn connect_default_voice_changed<F: Fn(&gio::Settings, &str) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_changed(Some("default-voice"), move |s, key| {
            f(s, key);
        })
    }
}

impl Deref for Settings {
    type Target = gio::Settings;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self(gio::Settings::new(APP_ID))
    }
}

unsafe impl Send for Settings {}
unsafe impl Sync for Settings {}
