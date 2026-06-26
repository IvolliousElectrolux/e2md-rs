use anyhow::{anyhow, bail, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::ChatResponse;
use crate::utils::{log_info, log_warn, record_usage, strip_markdown_fence};

#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    pub base_url: String,
    pub api_key: String,
    client: Client,
}

impl OpenAiProvider {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> anyhow::Result<Self> {
        let client = crate::utils::net::build_client()?;
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            client,
        })
    }
}

// ---- DTOs ----

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    response_format: ResponseFormat,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Deserialize)]
struct ChatCompletion {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Usage,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: Option<String>,
}

#[derive(Deserialize, Default)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: ApiError,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
    #[serde(default)]
    code: Option<String>,
}

// ---- Implementation ----

impl OpenAiProvider {
    pub async fn chat_async(
        &self,
        model: &str,
        system_prompt: &str,
        user_content: &str,
        max_tokens: Option<u32>,
    ) -> anyhow::Result<ChatResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = ChatRequest {
            model,
            messages: vec![
                Message { role: "system", content: system_prompt },
                Message { role: "user", content: user_content },
            ],
            stream: false,
            temperature: Some(0.1),
            max_tokens,
            response_format: ResponseFormat { kind: "text" },
        };

        let mut retries = 0u32;
        loop {
            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .context("HTTP request failed")?;

            let status = resp.status();
            let text = resp.text().await.context("Reading response body")?;

            match status.as_u16() {
                200 => {
                    let parsed: ChatCompletion = serde_json::from_str(&text)
                        .context("Parsing chat completion response")?;
                    let choice = parsed.choices.into_iter().next()
                        .ok_or_else(|| anyhow!("Empty choices array"))?;
                    let raw_content = choice.message.content.unwrap_or_default();
                    if raw_content.is_empty() {
                        log_warn("OpenAI returned empty content, using original chunk");
                    }
                    let content = strip_markdown_fence(&raw_content);
                    let finish = choice.finish_reason.unwrap_or_default();
                    if finish == "length" {
                        log_warn(format!("OpenAI finish_reason=length for model {model}"));
                    }
                    let usage = parsed.usage;
                    record_usage("openai", usage.total_tokens, 0.0);
                    log_info(format!(
                        "[OpenAI] {model} — tokens: {}", usage.total_tokens
                    ));
                    return Ok(ChatResponse {
                        content,
                        prompt_tokens: usage.prompt_tokens,
                        completion_tokens: usage.completion_tokens,
                        total_tokens: usage.total_tokens,
                        cost_usd: 0.0,
                        finish_reason: finish,
                    });
                }
                400 | 401 | 402 | 422 => {
                    let msg = parse_error_message(&text);
                    bail!("OpenAI non-retryable error {status}: {msg}");
                }
                429 => {
                    if text.contains("insufficient_quota") || text.contains("quota") {
                        bail!("OpenAI quota exceeded — please top up your account");
                    }
                    let wait = exponential_backoff(retries);
                    log_warn(format!("OpenAI 429 rate limit, retry {retries} in {wait}s"));
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                    retries += 1;
                    if retries > 4 {
                        bail!("OpenAI rate limit exceeded after {retries} retries");
                    }
                }
                code if code >= 500 => {
                    retries += 1;
                    if retries > 3 {
                        bail!("OpenAI server error {status} after {retries} retries");
                    }
                    let wait = exponential_backoff(retries);
                    log_warn(format!("OpenAI {status} error, retry {retries} in {wait}s"));
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                }
                other => {
                    bail!("OpenAI unexpected status {other}: {text}");
                }
            }
        }
    }
}

fn parse_error_message(text: &str) -> String {
    if let Ok(e) = serde_json::from_str::<ErrorBody>(text) {
        return e.error.message;
    }
    text.chars().take(200).collect()
}

fn exponential_backoff(retries: u32) -> u64 {
    match retries {
        0 => 1,
        1 => 3,
        2 => 6,
        _ => 12,
    }
}

#[async_trait::async_trait]
impl super::ChatProvider for OpenAiProvider {
    async fn chat(
        &self,
        model: &str,
        system_prompt: &str,
        user_content: &str,
        max_tokens: Option<u32>,
    ) -> anyhow::Result<ChatResponse> {
        self.chat_async(model, system_prompt, user_content, max_tokens).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_message_valid_json() {
        let json = r#"{"error":{"message":"invalid key","code":"invalid_api_key"}}"#;
        assert_eq!(parse_error_message(json), "invalid key");
    }

    #[test]
    fn exponential_backoff_values() {
        assert_eq!(exponential_backoff(0), 1);
        assert_eq!(exponential_backoff(1), 3);
        assert_eq!(exponential_backoff(2), 6);
        assert_eq!(exponential_backoff(5), 12);
    }
}
