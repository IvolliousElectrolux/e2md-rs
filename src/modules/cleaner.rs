use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::models::RuleDefinition;
use crate::providers::{DeepSeekProvider, OpenAiProvider, OpenRouterProvider};
use crate::utils::{
    file::ensure_dir,
    log::{log_error_tagged, log_info_tagged},
    markdown::{chunk_hash, split_markdown},
    path::job_tmp_dir,
};

pub struct CleanerModule;

impl CleanerModule {
    /// Run the full cleaning pipeline for a job.
    ///
    /// Reads `merged_raw.md`, splits it, runs each chunk through all rule stages,
    /// then merges the results into `cleaned.md`.
    pub async fn clean(
        guid: &str,
        rule: &RuleDefinition,
        config: &crate::models::AppConfig,
    ) -> Result<PathBuf> {
        let base = job_tmp_dir(guid);
        let raw_path = base.join("merged_raw.md");

        if !raw_path.exists() {
            bail!("找不到 merged_raw.md, 请先完成转换 ({})", guid);
        }

        let text = std::fs::read_to_string(&raw_path)?;
        let strategy = rule
            .chunk_strategy
            .as_ref()
            .map(|c| c.strategy_type.as_str())
            .unwrap_or("MaxTokens");
        let chunk_size = rule
            .chunk_strategy
            .as_ref()
            .map(|c| c.size)
            .unwrap_or(3000);

        let chunks = split_markdown(&text, strategy, chunk_size);
        log_info_tagged("clean", format!(
            "[Cleaner] {} 切分为 {} 个分片 (策略: {})",
            guid,
            chunks.len(),
            strategy
        ));

        // Load cache
        let cache_path = base.join("clean_cache.json");
        let mut cache: HashMap<String, String> = if cache_path.exists() {
            let data = std::fs::read_to_string(&cache_path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };

        let parts_dir = base.join("clean_md_parts");
        ensure_dir(&parts_dir)?;

        for (i, chunk) in chunks.iter().enumerate() {
            let hash = chunk_hash(chunk);
            let done_path = parts_dir.join(format!("chunk_{:04}_done.md", i));

            if done_path.exists() {
                log_info_tagged("clean", format!("[Cleaner] 跳过已完成分片 {}", i));
                continue;
            }

            if let Some(cached) = cache.get(&hash) {
                std::fs::write(&done_path, cached)?;
                log_info_tagged("clean", format!("[Cleaner] 命中缓存分片 {}", i));
                continue;
            }

            log_info_tagged("clean", format!("[Cleaner] 清洗分片 {}/{}", i + 1, chunks.len()));
            let mut current_text = chunk.clone();

            for stage in &rule.stages {
                let prompt = stage.prompt.replace("{{CONTENT}}", &current_text);
                let provider_name = stage.provider.to_ascii_lowercase();

                let result = match provider_name.as_str() {
                    "openai" => {
                        let p = OpenAiProvider::new(
                            &config.openai_base_url,
                            &config.openai_api_key,
                        )?;
                        p.chat_async(&stage.model, &prompt, &current_text, stage.max_tokens)
                            .await
                    }
                    "deepseek" => {
                        let p = DeepSeekProvider::new(
                            &config.deepseek_base_url,
                            &config.deepseek_api_key,
                        )?;
                        let thinking = stage.enable_thinking.unwrap_or(false);
                        p.chat_async(&stage.model, &prompt, &current_text, thinking, stage.max_tokens)
                            .await
                    }
                    _ => {
                        // Default to OpenRouter
                        let p = OpenRouterProvider::new(
                            &config.openrouter_base_url,
                            &config.openrouter_api_key,
                            if config.openrouter_referer.is_empty() {
                                None
                            } else {
                                Some(config.openrouter_referer.clone())
                            },
                            Some(config.openrouter_title.clone()),
                        )?;
                        p.chat_async(&stage.model, &prompt, &current_text, stage.max_tokens)
                            .await
                    }
                };

                match result {
                    Ok(resp) => {
                        current_text = resp.content;
                        log_info_tagged("clean", format!(
                            "[Cleaner] 阶段 '{}' 完成 — tokens: {}",
                            stage.name, resp.total_tokens
                        ));
                    }
                    Err(e) => {
                        log_error_tagged("clean", format!(
                            "[Cleaner] 分片 {} 阶段 '{}' 失败: {}",
                            i, stage.name, e
                        ));
                        // Fall back to original chunk on error
                    }
                }
            }

            std::fs::write(&done_path, &current_text)?;
            cache.insert(hash, current_text);

            // Save cache incrementally
            if let Ok(cache_json) = serde_json::to_string(&cache) {
                let _ = std::fs::write(&cache_path, cache_json);
            }
        }

        // Merge all done chunks
        let mut done_entries: Vec<_> = std::fs::read_dir(&parts_dir)?
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains("_done"))
            .collect();
        done_entries.sort_by_key(|e| e.file_name());

        let mut merged = String::new();
        for entry in &done_entries {
            let content = std::fs::read_to_string(entry.path())?;
            if !merged.is_empty() {
                merged.push_str("\n\n");
            }
            merged.push_str(content.trim());
        }

        let cleaned_path = base.join("cleaned.md");
        std::fs::write(&cleaned_path, &merged)?;
        log_info_tagged("clean", format!(
            "[Cleaner] 清洗完成: {} 个分片合并 -> cleaned.md ({})",
            done_entries.len(),
            guid
        ));

        Ok(cleaned_path)
    }
}
