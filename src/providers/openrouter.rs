use anyhow::{anyhow, bail, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::ChatResponse;
use crate::utils::{log_info_tagged, log_warn_tagged, record_usage, strip_markdown_fence};

#[derive(Debug, Clone)]
pub struct OpenRouterProvider {
    pub base_url: String,
    pub api_key: String,
    pub referer: Option<String>,
    pub app_title: Option<String>,
    client: Client,
}

impl OpenRouterProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        referer: Option<String>,
        app_title: Option<String>,
    ) -> anyhow::Result<Self> {
        let client = crate::utils::net::build_client()?;
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            referer,
            app_title,
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
    #[serde(default)]
    credits_charged: f64,
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

impl OpenRouterProvider {
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
        };

        let mut retries = 0u32;
        loop {
            let mut req = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json");

            if let Some(ref referer) = self.referer {
                req = req.header("HTTP-Referer", referer.as_str());
            }
            if let Some(ref title) = self.app_title {
                req = req.header("X-OpenRouter-Title", title.as_str());
            }

            let resp = req.json(&body).send().await.context("HTTP request failed")?;
            let status = resp.status();

            // Check Retry-After header before consuming body
            let retry_after: Option<u64> = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());

            let text = resp.text().await.context("Reading response body")?;

            match status.as_u16() {
                200 => {
                    let parsed: ChatCompletion = serde_json::from_str(&text)
                        .context("Parsing OpenRouter response")?;
                    let choice = parsed.choices.into_iter().next()
                        .ok_or_else(|| anyhow!("Empty choices array"))?;
                    let raw_content = choice.message.content.unwrap_or_default();
                    if raw_content.is_empty() {
                        log_warn_tagged("clean", format!("OpenRouter: empty content from {model}, credits_charged={}", parsed.credits_charged));
                    }
                    let content = strip_markdown_fence(&raw_content);
                    let finish = choice.finish_reason.unwrap_or_default();
                    if finish == "length" {
                        log_warn_tagged("clean", format!("OpenRouter finish_reason=length for {model}"));
                    }
                    record_usage("openrouter", parsed.usage.total_tokens, parsed.credits_charged);
                    log_info_tagged("clean", format!(
                        "[OpenRouter] {model} — tokens: {}, cost: ${:.5}",
                        parsed.usage.total_tokens, parsed.credits_charged
                    ));
                    return Ok(ChatResponse {
                        content,
                        prompt_tokens: parsed.usage.prompt_tokens,
                        completion_tokens: parsed.usage.completion_tokens,
                        total_tokens: parsed.usage.total_tokens,
                        cost_usd: parsed.credits_charged,
                        finish_reason: finish,
                    });
                }
                400 | 401 | 402 | 422 => {
                    let msg = parse_error(&text);
                    bail!("OpenRouter non-retryable {status}: {msg}");
                }
                429 => {
                    let wait = retry_after.unwrap_or_else(|| backoff(retries));
                    log_warn_tagged("clean", format!("OpenRouter 429 rate limit, retry in {wait}s"));
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                    retries += 1;
                    if retries > 5 {
                        bail!("OpenRouter rate limit exceeded after {retries} retries");
                    }
                }
                code if code >= 500 => {
                    retries += 1;
                    if retries > 3 {
                        bail!("OpenRouter server error {status} after retries");
                    }
                    tokio::time::sleep(Duration::from_secs(backoff(retries))).await;
                }
                other => bail!("OpenRouter unexpected {other}: {text}"),
            }
        }
    }
}

fn parse_error(text: &str) -> String {
    if let Ok(e) = serde_json::from_str::<ErrorBody>(text) {
        return e.error.message;
    }
    text.chars().take(200).collect()
}

fn backoff(n: u32) -> u64 {
    match n { 0 => 1, 1 => 3, 2 => 6, _ => 12 }
}

#[async_trait::async_trait]
impl super::ChatProvider for OpenRouterProvider {
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
    fn parse_error_json() {
        let j = r#"{"error":{"message":"model not found","code":"model_not_found"}}"#;
        assert_eq!(parse_error(j), "model not found");
    }

    #[test]
    fn backoff_values() {
        assert_eq!(backoff(0), 1);
        assert_eq!(backoff(1), 3);
        assert_eq!(backoff(2), 6);
        assert_eq!(backoff(10), 12);
    }
}
