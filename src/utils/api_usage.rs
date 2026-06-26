#![allow(dead_code)]
use parking_lot::Mutex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub tokens: u64,
    pub cost_usd: f64,
    pub calls: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AllUsage {
    pub openai: ProviderUsage,
    pub deepseek: ProviderUsage,
    pub openrouter: ProviderUsage,
}

static USAGE: Lazy<Mutex<AllUsage>> = Lazy::new(|| Mutex::new(AllUsage::default()));

pub fn record_usage(provider: &str, tokens: u64, cost_usd: f64) {
    let mut usage = USAGE.lock();
    let bucket = match provider.to_ascii_lowercase().as_str() {
        "openai" => &mut usage.openai,
        "deepseek" => &mut usage.deepseek,
        "openrouter" => &mut usage.openrouter,
        _ => &mut usage.openrouter,
    };
    bucket.tokens += tokens;
    bucket.cost_usd += cost_usd;
    bucket.calls += 1;
}

pub fn get_usage() -> AllUsage {
    USAGE.lock().clone()
}

pub fn reset_usage() {
    *USAGE.lock() = AllUsage::default();
}
