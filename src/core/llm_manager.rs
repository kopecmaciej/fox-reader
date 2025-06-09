use reqwest::Client;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use crate::{
    settings::{LLMProvider, ProviderConfig},
    SETTINGS,
};

const SYSTEM_PROMPT: &str = "You are a helpful assistant. You are talking to the user using voice so make your responses short and concise. Avoid using emojis, markdown, or other formatting, but if user asks for it, you can use them.";

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
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
}

impl LLMManager {
    pub fn reset_conversation(&self) {
        let mut history = self.conversation_history.lock().unwrap();
        history.clear();
    }

    pub fn add_user_message(&self, content: &str) {
        let mut history = self.conversation_history.lock().unwrap();
        history.push(Message::user(content));
    }

    pub fn add_assistant_message(&self, content: &str) {
        let mut history = self.conversation_history.lock().unwrap();
        history.push(Message::assistant(content));
    }

    fn get_active_config(&self) -> ProviderConfig {
        let settings = &SETTINGS;
        settings.get_active_provider_config()
    }

    pub async fn send_to_llm(
        &self,
        prompt: &str,
        language: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if self.conversation_history.lock().unwrap().len() == 1 {
            if let Some(first) = self.conversation_history.lock().unwrap().first_mut() {
                first.content = format!("Respond with language: {}. {}", language, first.content);
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

        let messages = build_messages_with_system(&history);

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

        let messages = build_messages_with_system(&history);

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
            "system": SYSTEM_PROMPT,
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

        let messages = build_messages_with_system(&history);

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

fn build_messages_with_system(history: &[Message]) -> Vec<Value> {
    let mut messages = vec![json!({ "role": "system", "content": SYSTEM_PROMPT })];
    messages.extend(history.iter().map(|msg| {
        json!({ "role": msg.role, "content": msg.content })
    }));
    messages
}
