use reqwest::Client;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use crate::{
    settings::{LLMProvider, ProviderConfig},
    SETTINGS,
};

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

#[derive(Default)]
pub struct LLMManager {
    client: Client,
    conversation_history: Arc<Mutex<Vec<Message>>>,
    system_prompt: String,
    conversation_language: String,
}

impl LLMManager {
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

    pub fn set_system_prompt(&mut self, prompt: &str) {
        self.system_prompt = prompt.to_string();
        self.reset_conversation();
    }

    pub fn set_active_provider(&mut self, provider: LLMProvider) {
        let settings = &SETTINGS;
        settings.set_active_provider(&provider.to_string());
    }

    fn get_active_config(&self) -> ProviderConfig {
        let settings = &SETTINGS;
        settings.get_active_provider_config()
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

        let settings = &SETTINGS;
        match settings.get_active_provider() {
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
        let provider_config = self.get_active_config();

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
            "model": provider_config.model,
            "messages": messages,
            "temperature": provider_config.temperature,
            "max_tokens": provider_config.max_tokens,
        });

        let url = provider_config.base_url.clone();

        let mut request = self
            .client
            .post(url)
            .header("Content-Type", "application/json");

        if let Some(api_key) = &provider_config.api_key {
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
        let provider_config = self.get_active_config();

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
            "model": provider_config.model,
            "messages": messages,
            "temperature": provider_config.temperature,
            "max_tokens": provider_config.max_tokens,
        });

        let url = provider_config.base_url.clone();

        let api_key = provider_config
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
        let provider_config = self.get_active_config();

        let mut messages = Vec::new();
        for msg in history {
            match msg.role.as_str() {
                "system" => {
                    messages.push(json!({
                        "role": "user",
                        "content": format!("<system>\n{}\n</system>", msg.content)
                    }));

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

        if messages.len() >= 2 {
            let second_msg = &messages[1];
            if second_msg["role"] == "assistant" && second_msg["content"] == "" {
                messages.remove(1);
            }
        }

        let request_body = json!({
            "model": provider_config.model,
            "messages": messages,
            "max_tokens": provider_config.max_tokens,
            "temperature": provider_config.temperature,
        });

        let url = provider_config.base_url.clone();

        let api_key = provider_config
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
        let provider_config = self.get_active_config();

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
            "model": provider_config.model,
            "messages": messages,
            "temperature": provider_config.temperature,
            "num_predict": provider_config.max_tokens,
            "stream": false,
        });

        let url = provider_config.base_url.clone();

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
