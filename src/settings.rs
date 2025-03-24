use adw::ColorScheme;
use gio::prelude::SettingsExt;
use gtk::{gdk::RGBA, gio, glib, pango::FontDescription};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, fs, ops::Deref, path::Path, sync::LazyLock};

use crate::APP_ID;
pub static SETTINGS: LazyLock<Settings> = LazyLock::new(Settings::default);

const WHISPER_MODELS: &[&str] = &[
    "tiny",
    "tiny.en",
    "tiny-q5_1",
    "tiny.en-q5_1",
    "tiny-q8_0",
    "base",
    "base.en",
    "base-q5_1",
    "base.en-q5_1",
    "base-q8_0",
    "small",
    "small.en",
    "small.en-tdrz",
    "small-q5_1",
    "small.en-q5_1",
    "small-q8_0",
    "medium",
    "medium.en",
    "medium-q5_0",
    "medium.en-q5_0",
    "medium-q8_0",
    "large-v1",
    "large-v2",
    "large-v2-q5_0",
    "large-v2-q8_0",
    "large-v3",
    "large-v3-q5_0",
    "large-v3-turbo",
    "large-v3-turbo-q5_0",
    "large-v3-turbo-q8_0",
];

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhisperModelDownloadStatus {
    NotDownloaded,
    Downloading(u8),
    Downloaded,
    Error(String),
}

impl Default for WhisperModelDownloadStatus {
    fn default() -> Self {
        Self::NotDownloaded
    }
}

impl Display for WhisperModelDownloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhisperModelDownloadStatus::NotDownloaded => write!(f, "Not downloaded"),
            WhisperModelDownloadStatus::Downloading(progress) => {
                write!(f, "Downloading: {}%", progress)
            }
            WhisperModelDownloadStatus::Downloaded => write!(f, "Downloaded"),
            WhisperModelDownloadStatus::Error(err) => write!(f, "Error: {}", err),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Settings(gio::Settings);

impl Settings {
    pub fn get_available_whisper_models() -> Vec<String> {
        WHISPER_MODELS.iter().map(|m| m.to_string()).collect()
    }

    pub fn get_whisper_model(&self) -> String {
        self.string("whisper-model").to_string()
    }

    pub fn set_whisper_model(&self, model: &str) {
        self.set_string("whisper-model", model)
            .expect("Failed to set Whisper model");
    }

    pub fn get_whisper_models_path(&self) -> String {
        let path = self.string("whisper-models-path").to_string();
        if path.is_empty() {
            let data_dir = glib::user_data_dir();
            let default_path = data_dir.join("whisper-models");

            if !default_path.exists() {
                let _ = fs::create_dir_all(&default_path);
            }

            let path_str = default_path.to_str().unwrap_or("").to_string();
            self.set_whisper_models_path(&path_str);
            return path_str;
        }
        path
    }

    pub fn set_whisper_models_path(&self, path: &str) {
        self.set_string("whisper-models-path", path)
            .expect("Failed to set Whisper models path");
    }

    pub fn get_whisper_model_path(&self) -> Option<String> {
        let model = self.get_whisper_model();
        let models_path = self.get_whisper_models_path();

        let model_path = Path::new(&models_path).join(format!("ggml-{}.bin", model));
        if model_path.exists() {
            return model_path.to_str().map(String::from);
        }
        None
    }

    pub fn is_whisper_model_downloaded(&self) -> bool {
        self.get_whisper_model_path().is_some()
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
