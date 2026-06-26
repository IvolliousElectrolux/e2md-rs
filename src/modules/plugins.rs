use crate::models::RuleDefinition;
use crate::utils::path::rules_dir;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

pub static RULES: Lazy<Mutex<Vec<RuleDefinition>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub struct PluginsModule;

impl PluginsModule {
    /// Scan `rules/` directory and load all `*.yaml` files.
    pub fn load_rules() -> Vec<RuleDefinition> {
        let dir = rules_dir();
        let mut rules = Vec::new();

        if !dir.exists() {
            // Create the directory and a minimal default rule on first run
            let _ = std::fs::create_dir_all(&dir);
            let default_rule = include_str!("../../rules/default.yaml");
            let _ = std::fs::write(dir.join("default.yaml"), default_rule);
        }

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                    continue;
                }
                match std::fs::read_to_string(&path) {
                    Ok(text) => match serde_yaml::from_str::<RuleDefinition>(&text) {
                        Ok(mut rule) => {
                            rule.file_name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string();
                            rules.push(rule);
                        }
                        Err(e) => {
                            crate::utils::log::log_warn(format!(
                                "解析规则文件失败 {}: {}",
                                path.display(),
                                e
                            ));
                        }
                    },
                    Err(e) => {
                        crate::utils::log::log_warn(format!(
                            "读取规则文件失败 {}: {}",
                            path.display(),
                            e
                        ));
                    }
                }
            }
        }

        *RULES.lock() = rules.clone();
        rules
    }

    pub fn available_rules() -> Vec<RuleDefinition> {
        RULES.lock().clone()
    }

    pub fn get_rule(file_name: &str) -> Option<RuleDefinition> {
        RULES
            .lock()
            .iter()
            .find(|r| r.file_name == file_name)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{RuleDefinition, StageDefinition};

    fn make_rule(name: &str, file: &str) -> RuleDefinition {
        RuleDefinition {
            name: name.to_string(),
            description: None,
            stages: vec![],
            chunk_strategy: None,
            file_name: file.to_string(),
        }
    }

    #[test]
    fn get_rule_finds_by_filename() {
        let rule = make_rule("Test Rule", "test.yaml");
        *RULES.lock() = vec![rule];
        let found = PluginsModule::get_rule("test.yaml");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Rule");
    }

    #[test]
    fn get_rule_returns_none_for_missing() {
        *RULES.lock() = vec![];
        assert!(PluginsModule::get_rule("nonexistent.yaml").is_none());
    }
}
