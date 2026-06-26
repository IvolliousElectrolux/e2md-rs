use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageDefinition {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Provider")]
    pub provider: String,
    #[serde(rename = "Model")]
    pub model: String,
    #[serde(rename = "Prompt")]
    pub prompt: String,
    #[serde(rename = "EnableThinking", default)]
    pub enable_thinking: Option<bool>,
    #[serde(rename = "MaxTokens", default)]
    pub max_tokens: Option<u32>,
    #[serde(rename = "Temperature", default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkStrategy {
    #[serde(rename = "Type")]
    pub strategy_type: String,
    #[serde(rename = "Size")]
    pub size: usize,
}

impl Default for ChunkStrategy {
    fn default() -> Self {
        Self {
            strategy_type: "MaxTokens".to_string(),
            size: 3000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDefinition {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "Stages")]
    pub stages: Vec<StageDefinition>,
    #[serde(rename = "ChunkStrategy")]
    pub chunk_strategy: Option<ChunkStrategy>,
    /// File name of the rule (e.g. "default.yaml")
    #[serde(skip)]
    pub file_name: String,
}
