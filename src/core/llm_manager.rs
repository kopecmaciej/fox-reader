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

pub struct LLMManager {
    client: Client,
    conversation_history: Arc<Mutex<Vec<Message>>>,
    system_prompt: String,
    conversation_language: String,
}

impl Default for LLMManager {
    fn default() -> Self {
        let system_prompt = "You are a helpful voice assistant. Respond in a conversational, natural way. Use short, clear sentences and avoid complex formatting, lists, or code. Keep responses concise and easy to listen to. Speak as if you're having a casual conversation. Use simple language that's easy to follow when heard rather than read.".to_string();

        Self {
            client: Client::new(),
            conversation_history: Arc::new(Mutex::new(vec![Message::system(&system_prompt)])),
            system_prompt,
            conversation_language: "en".to_string(),
        }
    }
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

    pub async fn send_to_lm_studio(
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
            "model": "model-identifier",
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 300,
        });

        let request = self
            .client
            .post("http://localhost:1234/v1/chat/completions")
            .header("Content-Type", "application/json")
            .json(&request_body);

        let response = request.send().await?;
        let response_json: Value = response.json().await?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Failed to parse response")
            .to_string();

        self.add_assistant_message(&content);

        Ok(content)
    }
}
