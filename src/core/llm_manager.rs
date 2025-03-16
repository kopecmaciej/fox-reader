use reqwest::Client;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LLMProvider {
    LMStudio,
    OpenAI,
    Anthropic,
    Ollama,
}

pub struct LLMConfig {
    pub provider: LLMProvider,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::LMStudio,
            api_key: None,
            base_url: Some("http://localhost:1234/v1".to_string()),
            model: "model-identifier".to_string(),
            temperature: 0.7,
            max_tokens: 300,
        }
    }
}

#[derive(Default)]
pub struct LLMManager {
    client: Client,
    conversation_history: Arc<Mutex<Vec<Message>>>,
    system_prompt: String,
    conversation_language: String,
    config: LLMConfig,
}

impl LLMManager {
    pub fn new(config: LLMConfig) -> Self {
        let system_prompt = "You are a helpful voice assistant. Respond in a conversational, natural way. Use short, clear sentences and avoid complex formatting, lists, or code. Keep responses concise and easy to listen to. Speak as if you're having a casual conversation. Use simple language that's easy to follow when heard rather than read.".to_string();

        Self {
            client: Client::new(),
            conversation_history: Arc::new(Mutex::new(vec![Message::system(&system_prompt)])),
            system_prompt,
            conversation_language: "en".to_string(),
            config,
        }
    }

    pub fn reset_conversation(&self) {
        let mut history = self.conversation_history.lock().unwrap();
        history.clear();
        history.push(Message::system(&self.system_prompt));
    }

    pub fn add_user_message(&self, content: &str) {
        let mut history = self.conversation_history.lock().unwrap();
        history.push(Message::user(content));
    }

    pub fn add_assistant_message(&self, content: &str) {
        let mut history = self.conversation_history.lock().unwrap();
        history.push(Message::assistant(content));
    }

    pub fn set_conversation_language(&mut self, language: &str) {
        self.conversation_language = language.to_string()
    }

    pub fn set_provider(&mut self, provider: LLMProvider) {
        self.config.provider = provider;
    }

    pub fn set_api_key(&mut self, api_key: &str) {
        self.config.api_key = Some(api_key.to_string());
    }

    pub fn set_base_url(&mut self, base_url: &str) {
        self.config.base_url = Some(base_url.to_string());
    }

    pub fn set_model(&mut self, model: &str) {
        self.config.model = model.to_string();
    }

    pub async fn send_to_llm(
        &self,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if self.conversation_history.lock().unwrap().len() == 1 {
            if let Some(first) = self.conversation_history.lock().unwrap().first_mut() {
                first.content = format!(
                    "Respond with language: {}. {}",
                    self.conversation_language, first.content
                );
            }
        }

        self.add_user_message(prompt);

        let history = {
            let history_guard = self.conversation_history.lock().unwrap();
            history_guard.clone()
        };

        match self.config.provider {
            LLMProvider::LMStudio => self.send_to_lm_studio(history).await,
            LLMProvider::OpenAI => self.send_to_openai(history).await,
            LLMProvider::Anthropic => self.send_to_anthropic(history).await,
            LLMProvider::Ollama => self.send_to_ollama(history).await,
        }
    }

    async fn send_to_lm_studio(
        &self,
        history: Vec<Message>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let messages = history
            .iter()
            .map(|msg| {
                json!({
                    "role": msg.role,
                    "content": msg.content
                })
            })
            .collect::<Vec<_>>();

        let request_body = json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
        });

        let base_url = self
            .config
            .base_url
            .clone()
            .unwrap_or(String::from("http://localhost:1234/v1"));
        let url = format!("{}/chat/completions", base_url);

        let mut request = self
            .client
            .post(url)
            .header("Content-Type", "application/json");

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.json(&request_body).send().await?;
        let response_json: Value = response.json().await?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Failed to parse response")
            .to_string();

        self.add_assistant_message(&content);

        Ok(content)
    }

    async fn send_to_openai(
        &self,
        history: Vec<Message>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let messages = history
            .iter()
            .map(|msg| {
                json!({
                    "role": msg.role,
                    "content": msg.content
                })
            })
            .collect::<Vec<_>>();

        let request_body = json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
        });

        let url = "https://api.openai.com/v1/chat/completions";

        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or("OpenAI API key is required")?;

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request_body)
            .send()
            .await?;

        let response_json: Value = response.json().await?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Failed to parse response")
            .to_string();

        self.add_assistant_message(&content);

        Ok(content)
    }

    async fn send_to_anthropic(
        &self,
        history: Vec<Message>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Convert to Anthropic message format
        let mut messages = Vec::new();
        for msg in history {
            match msg.role.as_str() {
                "system" => {
                    // Anthropic handles system prompts differently
                    messages.push(json!({
                        "role": "user",
                        "content": format!("<system>\n{}\n</system>", msg.content)
                    }));

                    // Add a placeholder assistant response after the system prompt
                    messages.push(json!({
                        "role": "assistant",
                        "content": ""
                    }));
                }
                "user" | "assistant" => {
                    messages.push(json!({
                        "role": msg.role,
                        "content": msg.content
                    }));
                }
                _ => return Err("Invalid message role for Anthropic".into()),
            }
        }

        // Remove the first empty assistant message if it exists
        if messages.len() >= 2 {
            let second_msg = &messages[1];
            if second_msg["role"] == "assistant" && second_msg["content"] == "" {
                messages.remove(1);
            }
        }

        let request_body = json!({
            "model": self.config.model,
            "messages": messages,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
        });

        let url = "https://api.anthropic.com/v1/messages";

        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or("Anthropic API key is required")?;

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await?;

        let response_json: Value = response.json().await?;

        let content = response_json["content"][0]["text"]
            .as_str()
            .unwrap_or("Failed to parse response")
            .to_string();

        self.add_assistant_message(&content);

        Ok(content)
    }

    async fn send_to_ollama(
        &self,
        history: Vec<Message>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let messages = history
            .iter()
            .map(|msg| {
                json!({
                    "role": msg.role,
                    "content": msg.content
                })
            })
            .collect::<Vec<_>>();

        let request_body = json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": self.config.temperature,
            "num_predict": self.config.max_tokens,
            "stream": false,
        });

        let base_url = self
            .config
            .base_url
            .clone()
            .unwrap_or("http://localhost:11434/api".to_string());

        let url = format!("{}/chat", base_url);

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_json: Value = response.json().await?;

        let content = response_json["message"]["content"]
            .as_str()
            .unwrap_or("Failed to parse response")
            .to_string();

        self.add_assistant_message(&content);

        Ok(content)
    }
}
