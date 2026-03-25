use futures::stream::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct AiClient {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Option<Message>,
    delta: Option<Delta>,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
}

impl AiClient {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        // Normalize base_url - remove trailing slash
        let normalized_url = base_url.trim_end_matches('/');

        Self {
            client,
            base_url: normalized_url.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    pub async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<impl Stream<Item = Result<String, String>>, String> {
        let url = format!("{}/chat/completions", self.base_url);

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            temperature: Some(0.7),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                eprintln!("HTTP request failed. URL: {}, Error: {:?}", url, e);
                if e.is_timeout() {
                    format!("Request timeout. Check your internet connection.")
                } else if e.is_connect() {
                    format!("Connection failed. Cannot reach {}", url)
                } else if e.is_request() {
                    format!("Request error: {}", e)
                } else {
                    format!("Network error: {}", e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("API error {}: {}", status, error_text));
        }

        let stream = response
            .bytes_stream()
            .filter_map(|chunk_result| {
                futures::future::ready(match chunk_result {
                    Ok(chunk) => {
                        // Parse SSE format: "data: {...}\n\n"
                        let text = String::from_utf8_lossy(&chunk);
                        let mut result = String::new();

                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let json_str = &line[6..]; // Skip "data: "

                                if json_str == "[DONE]" {
                                    break;
                                }

                                if let Ok(response) = serde_json::from_str::<ChatResponse>(json_str) {
                                    if let Some(choice) = response.choices.first() {
                                        if let Some(delta) = &choice.delta {
                                            if let Some(content) = &delta.content {
                                                result.push_str(content);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Only emit non-empty chunks
                        if !result.is_empty() {
                            Some(Ok(result))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(format!("Stream error: {}", e))),
                })
            });

        Ok(stream)
    }

    pub async fn chat_complete(&self, messages: Vec<ChatMessage>) -> Result<String, String> {
        let url = format!("{}/chat/completions", self.base_url);

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
            temperature: Some(0.7),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(60))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                eprintln!("HTTP request failed (chat_complete). URL: {}, Error: {:?}", url, e);
                if e.is_timeout() {
                    format!("Request timeout (60s). Check your internet connection.")
                } else if e.is_connect() {
                    format!("Connection failed. Cannot reach {}", url)
                } else {
                    format!("Network error: {}", e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("API error {}: {}", status, error_text));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        chat_response
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .map(|m| m.content.clone())
            .ok_or_else(|| "No content in response".to_string())
    }

    pub async fn test_connection(&self) -> Result<String, String> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "ping".to_string(),
        }];

        self.chat_complete(messages).await.map(|_| self.model.clone())
    }
}
