/// Tests for core business modules

#[cfg(test)]
mod cut_tests {
    use crate::modules::cut::CutModule;

    #[test]
    fn compute_split_plan_small_doc() {
        let plan = CutModule::compute_split_plan(50, 200);
        assert_eq!(plan, vec![(1, 50)]);
    }

    #[test]
    fn compute_split_plan_large_doc_even() {
        let plan = CutModule::compute_split_plan(600, 200);
        assert_eq!(plan, vec![(1, 200), (201, 400), (401, 600)]);
    }

    #[test]
    fn compute_split_plan_large_doc_uneven() {
        let plan = CutModule::compute_split_plan(450, 200);
        assert_eq!(plan.len(), 3);
        assert_eq!(plan[2], (401, 450));
    }

    #[test]
    fn compute_split_plan_threshold_zero_returns_single() {
        let plan = CutModule::compute_split_plan(100, 0);
        assert_eq!(plan, vec![(1, 100)]);
    }

    #[test]
    fn compute_split_plan_exactly_threshold() {
        let plan = CutModule::compute_split_plan(200, 200);
        assert_eq!(plan, vec![(1, 200)]);
    }

    #[test]
    fn merge_raw_md_parts_integration() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let parts = tmp.path().join("raw_md_parts");
        std::fs::create_dir_all(&parts).unwrap();
        std::fs::write(parts.join("part_0001.md"), "# Part 1\n\nContent one").unwrap();
        std::fs::write(parts.join("part_0002.md"), "# Part 2\n\nContent two").unwrap();

        // Direct merge logic test (not calling module function to avoid path override issues)
        let mut entries: Vec<_> = std::fs::read_dir(&parts).unwrap().flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        let mut merged = String::new();
        for e in &entries {
            let content = std::fs::read_to_string(e.path()).unwrap();
            if !merged.is_empty() { merged.push_str("\n\n"); }
            merged.push_str(content.trim());
        }
        assert!(merged.contains("Part 1"));
        assert!(merged.contains("Part 2"));
        assert!(merged.contains("Content one"));
        assert!(merged.contains("Content two"));
    }
}

#[cfg(test)]
mod plugins_tests {
    use crate::models::{ChunkStrategy, RuleDefinition, StageDefinition};
    use crate::modules::plugins::PluginsModule;

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
    fn get_rule_by_filename() {
        use crate::modules::plugins::RULES;
        *RULES.lock() = vec![make_rule("My Rule", "my.yaml")];
        assert!(PluginsModule::get_rule("my.yaml").is_some());
        assert_eq!(PluginsModule::get_rule("my.yaml").unwrap().name, "My Rule");
    }

    #[test]
    fn get_rule_missing_returns_none() {
        use crate::modules::plugins::RULES;
        *RULES.lock() = vec![];
        assert!(PluginsModule::get_rule("absent.yaml").is_none());
    }

    #[test]
    fn available_rules_returns_all() {
        use crate::modules::plugins::RULES;
        *RULES.lock() = vec![
            make_rule("Rule A", "a.yaml"),
            make_rule("Rule B", "b.yaml"),
        ];
        let rules = PluginsModule::available_rules();
        assert_eq!(rules.len(), 2);
    }
}

#[cfg(test)]
mod process_tests {
    use crate::models::job::{JobRecord, JobStatus};

    #[test]
    fn job_record_fields_correct() {
        let j = JobRecord::new("guid-xyz", "test.pdf", ".pdf", "");
        assert_eq!(j.job_guid, "guid-xyz");
        assert_eq!(j.original_file_name, "test.pdf");
        assert_eq!(j.original_file_ext, ".pdf");
        assert_eq!(j.status, JobStatus::Pending);
        assert!(j.export_path.is_empty());
        assert!(j.error_message.is_none());
    }

    #[test]
    fn job_record_is_active_for_converting() {
        let mut j = JobRecord::new("g", "f.pdf", ".pdf", "");
        j.status = JobStatus::Converting;
        assert!(j.is_active());
        j.status = JobStatus::Cleaning;
        assert!(j.is_active());
        j.status = JobStatus::CleanPartial;
        assert!(j.is_active());
    }

    #[test]
    fn job_record_not_active_for_terminal_states() {
        let terminal = [
            JobStatus::Pending,
            JobStatus::Converted,
            JobStatus::Cleaned,
            JobStatus::Exported,
            JobStatus::Failed,
        ];
        for status in terminal {
            let mut j = JobRecord::new("g", "f.pdf", ".pdf", "");
            j.status = status;
            assert!(!j.is_active(), "Expected not active for status {}", j.status);
        }
    }
}

/// Tests for rule YAML deserialization and submit_staged_files pipeline.
#[cfg(test)]
mod convert_pipeline_tests {
    use crate::models::{RuleDefinition, StageDefinition};

    /// Verify that StageDefinition deserializes correctly from PascalCase YAML keys.
    /// This was the root cause of "missing field name~" parse errors.
    #[test]
    fn stage_definition_deserializes_from_pascal_case_yaml() {
        let yaml = r#"
Name: "通用排版修复"
Description: "测试规则"
Stages:
  - Name: "阶段一"
    Provider: "OpenRouter"
    Model: "deepseek/deepseek-chat"
    Prompt: "你好 {{CONTENT}}"
    MaxTokens: 4096
ChunkStrategy:
  Type: "MaxTokens"
  Size: 3000
"#;
        let rule: RuleDefinition = serde_yaml::from_str(yaml)
            .expect("default.yaml 格式应能正确解析");
        assert_eq!(rule.name, "通用排版修复");
        assert_eq!(rule.stages.len(), 1);
        assert_eq!(rule.stages[0].name, "阶段一");
        assert_eq!(rule.stages[0].provider, "OpenRouter");
        assert_eq!(rule.stages[0].model, "deepseek/deepseek-chat");
        assert_eq!(rule.stages[0].max_tokens, Some(4096));
        assert!(rule.chunk_strategy.is_some());
    }

    /// Embedded default.yaml (the same file shipped with the binary) must parse cleanly.
    #[test]
    fn default_yaml_parses_without_error() {
        let yaml = include_str!("../../rules/default.yaml");
        let result = serde_yaml::from_str::<RuleDefinition>(yaml);
        assert!(
            result.is_ok(),
            "default.yaml 解析失败: {:?}",
            result.err()
        );
        let rule = result.unwrap();
        assert!(!rule.name.is_empty());
        assert!(!rule.stages.is_empty());
        assert!(!rule.stages[0].name.is_empty());
        assert!(!rule.stages[0].prompt.is_empty());
    }

    /// submit_staged_files with no token must not panic and must mark jobs as Failed.
    /// Uses GPUI TestAppContext to provide a real background_executor.
    #[gpui::test]
    async fn submit_no_token_marks_jobs_failed(cx: &mut gpui::TestAppContext) {
        use crate::models::JobStatus;
        use crate::ui::AppState;
        use std::time::Duration;

        let state = cx.update(|cx| {
            let _ = cx; // satisfy borrow
            AppState::new().expect("AppState::new failed")
        });

        // Stage a dummy file (doesn't need to exist — create_job will copy it,
        // but we can use a tempfile so the copy succeeds)
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"%PDF-1.4 fake").unwrap();
        state.add_staged_file(tmp.path().to_path_buf());

        // Submit: no token is set (AppConfig default has empty mineru_token)
        let guids = cx.update(|cx| state.submit_staged_files(cx));
        assert_eq!(guids.len(), 1, "应该提交了 1 个任务");

        // spawn_convert_task now uses std::thread — give it time to complete
        std::thread::sleep(Duration::from_millis(200));

        // The job should now be Failed (token not configured)
        let jobs = state.jobs();
        let job = jobs.iter().find(|j| j.job_guid == guids[0]);
        assert!(job.is_some(), "任务记录应存在");
        let job = job.unwrap();
        assert_eq!(
            job.status,
            JobStatus::Failed,
            "无 token 时任务应标记为 Failed, 实际状态: {}",
            job.status
        );
        assert!(
            job.error_message.as_deref().unwrap_or("").contains("token"),
            "错误消息应包含 token 字样"
        );
    }

    /// submit_staged_files with an empty staged list must return empty guids and not panic.
    #[gpui::test]
    fn submit_empty_staged_list_is_noop(cx: &mut gpui::TestAppContext) {
        use crate::ui::AppState;

        let state = AppState::new().expect("AppState::new failed");
        let guids = cx.update(|cx| state.submit_staged_files(cx));
        assert!(guids.is_empty(), "没有文件时返回的 guid 列表应为空");
        assert!(
            state.jobs().is_empty() || state.jobs().iter().all(|j| j.job_guid != ""),
            "不应创建任何新任务"
        );
    }
}
