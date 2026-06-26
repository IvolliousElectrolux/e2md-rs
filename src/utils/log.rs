#![allow(dead_code)]
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Success,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
    /// Optional tag to separate logs by context (e.g. "convert", "clean").
    pub tag: Option<String>,
}

impl LogEntry {
    pub fn info(msg: impl Into<String>) -> Self {
        Self { level: LogLevel::Info, message: msg.into(), timestamp: chrono::Local::now(), tag: None }
    }
    pub fn warn(msg: impl Into<String>) -> Self {
        Self { level: LogLevel::Warn, message: msg.into(), timestamp: chrono::Local::now(), tag: None }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self { level: LogLevel::Error, message: msg.into(), timestamp: chrono::Local::now(), tag: None }
    }
    pub fn success(msg: impl Into<String>) -> Self {
        Self { level: LogLevel::Success, message: msg.into(), timestamp: chrono::Local::now(), tag: None }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn format(&self) -> String {
        format!("[{}] {}", self.timestamp.format("%H:%M:%S"), self.message)
    }
}

/// Emit a log entry scoped to a specific context tag.
pub fn emit_tagged(entry: LogEntry, tag: &str) {
    emit(entry.with_tag(tag));
}

pub fn log_info_tagged(tag: &str, msg: impl Into<String>) {
    emit_tagged(LogEntry::info(msg), tag);
}
pub fn log_warn_tagged(tag: &str, msg: impl Into<String>) {
    emit_tagged(LogEntry::warn(msg), tag);
}
pub fn log_error_tagged(tag: &str, msg: impl Into<String>) {
    emit_tagged(LogEntry::error(msg), tag);
}
pub fn log_success_tagged(tag: &str, msg: impl Into<String>) {
    emit_tagged(LogEntry::success(msg), tag);
}

/// Return log entries filtered by tag, most-recent first.
pub fn history_tagged(tag: &str) -> Vec<LogEntry> {
    LOG_HISTORY
        .lock()
        .iter()
        .filter(|e| e.tag.as_deref() == Some(tag))
        .cloned()
        .collect()
}

type LogCallback = Box<dyn Fn(LogEntry) + Send + Sync>;

static LISTENERS: Lazy<Mutex<Vec<Arc<LogCallback>>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

static LOG_HISTORY: Lazy<Mutex<Vec<LogEntry>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

pub fn subscribe(cb: impl Fn(LogEntry) + Send + Sync + 'static) {
    let mut listeners = LISTENERS.lock();
    listeners.push(Arc::new(Box::new(cb)));
}

pub fn emit(entry: LogEntry) {
    {
        let mut history = LOG_HISTORY.lock();
        history.push(entry.clone());
        if history.len() > 500 {
            history.remove(0);
        }
    }
    let listeners = LISTENERS.lock();
    for cb in listeners.iter() {
        cb(entry.clone());
    }
}

pub fn history() -> Vec<LogEntry> {
    LOG_HISTORY.lock().clone()
}

pub fn log_info(msg: impl Into<String>) {
    emit(LogEntry::info(msg));
}

pub fn log_warn(msg: impl Into<String>) {
    emit(LogEntry::warn(msg));
}

pub fn log_error(msg: impl Into<String>) {
    emit(LogEntry::error(msg));
}

pub fn log_success(msg: impl Into<String>) {
    emit(LogEntry::success(msg));
}

#[derive(Debug, Default, Clone)]
pub struct ApiUsageStat {
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub call_count: u64,
}

static API_USAGE: Lazy<Mutex<ApiUsageStat>> =
    Lazy::new(|| Mutex::new(ApiUsageStat::default()));

pub fn record_api_usage(tokens: u64, cost_usd: f64) {
    let mut usage = API_USAGE.lock();
    usage.total_tokens += tokens;
    usage.total_cost_usd += cost_usd;
    usage.call_count += 1;
}

pub fn get_api_usage() -> ApiUsageStat {
    API_USAGE.lock().clone()
}

/// Test helper to reset API usage counter.
#[cfg(test)]
pub fn reset_api_usage_for_test() {
    *API_USAGE.lock() = ApiUsageStat::default();
}
