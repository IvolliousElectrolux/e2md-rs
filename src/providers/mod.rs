#![allow(dead_code, unused_imports)]

pub mod openai;
pub mod deepseek;
pub mod openrouter;

pub use openai::OpenAiProvider;
pub use deepseek::DeepSeekProvider;
pub use openrouter::OpenRouterProvider;

use anyhow::Result;

/// Common interface for all chat-completion providers.
#[async_trait::async_trait]
pub trait ChatProvider: Send + Sync {
    async fn chat(
        &self,
        model: &str,
        system_prompt: &str,
        user_content: &str,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse>;
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
    pub finish_reason: String,
}
