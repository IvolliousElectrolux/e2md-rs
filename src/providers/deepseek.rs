use anyhow::{anyhow, bail, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::ChatResponse;
use crate::utils::{log_warn, record_usage, strip_markdown_fence, log_info};

#[derive(Debug, Clone)]
pub struct DeepSeekProvider {
    pub base_url: String,
    pub api_key: String,
    client: Client,
}

impl DeepSeekProvider {
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
struct ThinkingConfig<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
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
    #[allow(dead_code)]
    reasoning_content: Option<String>,
}

#[derive(Deserialize, Default)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

// ---- Implementation ----

impl DeepSeekProvider {
    pub async fn chat_async(
        &self,
        model: &str,
        system_prompt: &str,
        user_content: &str,
        enable_thinking: bool,
        max_tokens: Option<u32>,
    ) -> anyhow::Result<ChatResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let body = if enable_thinking {
            ChatRequest {
                model,
                messages: vec![
                    Message { role: "system", content: system_prompt },
                    Message { role: "user", content: user_content },
                ],
                stream: false,
                thinking: Some(ThinkingConfig { kind: "enabled" }),
                reasoning_effort: Some("high"),
                temperature: None,
                max_tokens,
            }
        } else {
            ChatRequest {
                model,
                messages: vec![
                    Message { role: "system", content: system_prompt },
                    Message { role: "user", content: user_content },
                ],
                stream: false,
                thinking: Some(ThinkingConfig { kind: "disabled" }),
                reasoning_effort: None,
                temperature: Some(0.1),
                max_tokens,
            }
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
                        .context("Parsing DeepSeek response")?;
                    let choice = parsed.choices.into_iter().next()
                        .ok_or_else(|| anyhow!("Empty choices array"))?;
                    let raw_content = choice.message.content.unwrap_or_default();
                    let content = strip_markdown_fence(&raw_content);
                    let finish = choice.finish_reason.unwrap_or_default();
                    if finish == "length" {
                        log_warn(format!("DeepSeek finish_reason=length for model {model}"));
                    }
                    let usage = parsed.usage;
                    record_usage("deepseek", usage.total_tokens, 0.0);
                    log_info(format!(
                        "[DeepSeek] {model} — tokens: {}", usage.total_tokens
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
                    bail!("DeepSeek non-retryable error {status}: {}", text.chars().take(300).collect::<String>());
                }
                429 => {
                    let wait = backoff(retries);
                    log_warn(format!("DeepSeek 429 rate limit, retry {retries} in {wait}s"));
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                    retries += 1;
                    if retries > 4 {
                        bail!("DeepSeek rate limit exceeded after {retries} retries");
                    }
                }
                code if code >= 500 => {
                    retries += 1;
                    if retries > 3 {
                        bail!("DeepSeek server error {status} after {retries} retries");
                    }
                    tokio::time::sleep(Duration::from_secs(backoff(retries))).await;
                }
                other => bail!("DeepSeek unexpected status {other}: {text}"),
            }
        }
    }
}

fn backoff(retries: u32) -> u64 {
    match retries { 0 => 1, 1 => 3, 2 => 6, _ => 12 }
}

#[async_trait::async_trait]
impl super::ChatProvider for DeepSeekProvider {
    async fn chat(
        &self,
        model: &str,
        system_prompt: &str,
        user_content: &str,
        max_tokens: Option<u32>,
    ) -> anyhow::Result<ChatResponse> {
        self.chat_async(model, system_prompt, user_content, false, max_tokens).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_sequence() {
        assert_eq!(backoff(0), 1);
        assert_eq!(backoff(1), 3);
        assert_eq!(backoff(2), 6);
    }
}
