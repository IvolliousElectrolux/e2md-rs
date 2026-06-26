use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CONFIG_FILE: &str = "e2md.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub mineru_token: String,
    pub mineru_base_url: String,

    pub openai_api_key: String,
    pub openai_base_url: String,

    pub deepseek_api_key: String,
    pub deepseek_base_url: String,

    pub openrouter_api_key: String,
    pub openrouter_base_url: String,
    pub openrouter_referer: String,
    pub openrouter_title: String,

    pub max_convert_concurrency: usize,
    pub max_clean_concurrency: usize,

    pub pdf_split_threshold_pages: u32,
    pub pdf_split_enabled: bool,

    pub md_chunk_size: usize,
    pub md_split_enabled: bool,

    pub export_directory: String,
    pub auto_clean_after_convert: bool,
    pub default_rule: String,

    /// Active theme name, e.g. "Default Light", "Default Dark", "Dracula", "Catppuccin Mocha", "Nord", "Solarized Light"
    #[serde(default = "default_theme_name")]
    pub theme_name: String,

    /// Last saved window geometry (restore bounds, pixels).  None = use default / center.
    #[serde(default)]
    pub window_x: Option<f32>,
    #[serde(default)]
    pub window_y: Option<f32>,
    #[serde(default)]
    pub window_width: Option<f32>,
    #[serde(default)]
    pub window_height: Option<f32>,
    /// Whether the window was maximized when last closed.
    #[serde(default)]
    pub window_maximized: bool,
    /// Whether the window was fullscreen when last closed.
    #[serde(default)]
    pub window_fullscreen: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mineru_token: String::new(),
            mineru_base_url: "https://mineru.net".to_string(),
            openai_api_key: String::new(),
            openai_base_url: "https://api.openai.com".to_string(),
            deepseek_api_key: String::new(),
            deepseek_base_url: "https://api.deepseek.com".to_string(),
            openrouter_api_key: String::new(),
            openrouter_base_url: "https://openrouter.ai/api".to_string(),
            openrouter_referer: String::new(),
            openrouter_title: "E2MD".to_string(),
            max_convert_concurrency: 50,
            max_clean_concurrency: 3,
            pdf_split_threshold_pages: 200,
            pdf_split_enabled: true,
            md_chunk_size: 3000,
            md_split_enabled: false,
            export_directory: "export".to_string(),
            auto_clean_after_convert: false,
            default_rule: "default.yaml".to_string(),
            theme_name: default_theme_name(),
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
            window_maximized: false,
            window_fullscreen: false,
        }
    }
}

fn default_theme_name() -> String {
    "Default Light".to_string()
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path();
        if path.exists() {
            let text = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&text)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }
}

fn config_path() -> PathBuf {
    // Prefer executable directory, fall back to current dir
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join(CONFIG_FILE);
        }
    }
    PathBuf::from(CONFIG_FILE)
}
