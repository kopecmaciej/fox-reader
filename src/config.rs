use adw::ColorScheme;
use gtk::gdk::RGBA;
use gtk::pango::FontDescription;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, fmt::Display, rc::Rc};

use crate::{core::file_handler::FileHandler, paths::get_app_config_path};

const OLLAMA_URL: &str = "http://localhost:11434/api/chat";
const LMSTUDIO_URL: &str = "http://localhost:1234/v1/chat/completions";
const OPENAI_URL: &str = "https://api.openai.com/v1/chat/completions";
const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";

pub type SharedConfig = Rc<RefCell<UserConfig>>;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        ProviderConfig::new(LLMProvider::Ollama)
    }
}

impl ProviderConfig {
    fn new(provider: LLMProvider) -> Self {
        let (base_url, model) = match provider {
            LLMProvider::LMStudio => (LMSTUDIO_URL, ""),
            LLMProvider::Ollama => (OLLAMA_URL, ""),
            LLMProvider::OpenAI => (OPENAI_URL, "gpt-4o-mini"),
            LLMProvider::Anthropic => (ANTHROPIC_URL, "claude-3-5-haiku-latest"),
        };

        Self {
            api_key: None,
            base_url: String::from(base_url),
            model: Some(String::from(model)),
            temperature: None,
            max_tokens: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LLMConfig {
    pub active_provider: LLMProvider,
    pub providers: HashMap<LLMProvider, ProviderConfig>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        let mut providers = HashMap::new();

        providers.insert(
            LLMProvider::Ollama,
            ProviderConfig::new(LLMProvider::Ollama),
        );
        providers.insert(
            LLMProvider::LMStudio,
            ProviderConfig::new(LLMProvider::LMStudio),
        );
        providers.insert(
            LLMProvider::OpenAI,
            ProviderConfig::new(LLMProvider::OpenAI),
        );
        providers.insert(
            LLMProvider::Anthropic,
            ProviderConfig::new(LLMProvider::Anthropic),
        );

        Self {
            active_provider: LLMProvider::LMStudio,
            providers,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserConfig {
    pub font: Option<String>,
    pub highlight_color: String,
    pub theme: String,
    pub llm_config: LLMConfig,
}

impl Default for UserConfig {
    fn default() -> Self {
        let initial_rgba = gtk::gdk::RGBA::new(1.0, 1.0, 0.0, 0.3);
        Self {
            font: None,
            highlight_color: initial_rgba.to_string(),
            theme: "light".to_string(),
            llm_config: LLMConfig::default(),
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
        RGBA::parse(self.highlight_color.clone()).unwrap_or(initial_rgba)
    }

    pub fn get_color_scheme(&self) -> ColorScheme {
        match self.theme.as_str() {
            "light" => ColorScheme::ForceLight,
            "dark" => ColorScheme::ForceDark,
            _ => ColorScheme::Default,
        }
    }

    pub fn is_dark_color_scheme(&self) -> bool {
        self.get_color_scheme() == ColorScheme::ForceDark
    }

    pub fn set_font(&mut self, font_desc: &FontDescription) {
        self.font = Some(font_desc.to_string());
        if let Err(e) = self.save() {
            eprintln!("Failed to save font settings: {}", e);
        }
    }

    pub fn set_highlight_color(&mut self, rgba: &RGBA) {
        self.highlight_color = rgba.to_string();
        if let Err(e) = self.save() {
            eprintln!("Failed to save highlight color settings: {}", e);
        }
    }

    pub fn set_theme(&mut self, is_dark: bool) {
        self.theme = if is_dark { "dark" } else { "light" }.to_string();
        if let Err(e) = self.save() {
            eprintln!("Failed to save theme settings: {}", e);
        }
    }

    pub fn set_active_llm_provider(&mut self, provider: LLMProvider) {
        self.llm_config.active_provider = provider.clone();

        // Ensure the provider exists in the map
        if !self.llm_config.providers.contains_key(&provider) {
            self.llm_config
                .providers
                .insert(provider.clone(), ProviderConfig::new(provider));
        }

        if let Err(e) = self.save() {
            eprintln!("Failed to save active LLM provider: {}", e);
        }
    }

    pub fn get_active_provider_config(&self) -> ProviderConfig {
        self.llm_config
            .providers
            .get(&self.llm_config.active_provider)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_llm_api_key(&mut self, api_key: String) {
        if let Some(config) = self
            .llm_config
            .providers
            .get_mut(&self.llm_config.active_provider)
        {
            config.api_key = Some(api_key);
            if let Err(e) = self.save() {
                eprintln!("Failed to save LLM API key settings: {}", e);
            }
        }
    }

    pub fn set_llm_base_url(&mut self, base_url: String) {
        if let Some(config) = self
            .llm_config
            .providers
            .get_mut(&self.llm_config.active_provider)
        {
            config.base_url = base_url;
            if let Err(e) = self.save() {
                eprintln!("Failed to save LLM base URL settings: {}", e);
            }
        }
    }

    pub fn set_llm_model(&mut self, model: String) {
        if let Some(config) = self
            .llm_config
            .providers
            .get_mut(&self.llm_config.active_provider)
        {
            config.model = Some(model);
            if let Err(e) = self.save() {
                eprintln!("Failed to save LLM model settings: {}", e);
            }
        }
    }

    pub fn set_llm_temperature(&mut self, temperature: f32) {
        if let Some(config) = self
            .llm_config
            .providers
            .get_mut(&self.llm_config.active_provider)
        {
            config.temperature = Some(temperature);
            if let Err(e) = self.save() {
                eprintln!("Failed to save LLM temperature settings: {}", e);
            }
        }
    }

    pub fn set_llm_max_tokens(&mut self, max_tokens: u32) {
        if let Some(config) = self
            .llm_config
            .providers
            .get_mut(&self.llm_config.active_provider)
        {
            config.max_tokens = Some(max_tokens);
            if let Err(e) = self.save() {
                eprintln!("Failed to save LLM max tokens settings: {}", e);
            }
        }
    }

    // Get a specific provider's configuration
    pub fn get_provider_config(&self, provider: &LLMProvider) -> ProviderConfig {
        self.llm_config
            .providers
            .get(provider)
            .cloned()
            .unwrap_or_default()
    }

    // Directly set a provider's full configuration
    pub fn set_provider_config(&mut self, provider: LLMProvider, config: ProviderConfig) {
        self.llm_config.providers.insert(provider, config);
        if let Err(e) = self.save() {
            eprintln!("Failed to save provider configuration: {}", e);
        }
    }
}
