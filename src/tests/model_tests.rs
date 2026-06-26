/// Tests for data models

#[cfg(test)]
mod job_tests {
    use crate::models::job::*;

    #[test]
    fn new_job_has_pending_status() {
        let job = JobRecord::new("abc", "test.pdf", ".pdf", "");
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[test]
    fn active_statuses() {
        let mut job = JobRecord::new("g", "f.pdf", ".pdf", "");
        job.status = JobStatus::Converting;
        assert!(job.is_active());
        job.status = JobStatus::Cleaning;
        assert!(job.is_active());
        job.status = JobStatus::Pending;
        assert!(!job.is_active());
        job.status = JobStatus::Exported;
        assert!(!job.is_active());
    }

    #[test]
    fn done_statuses() {
        let mut job = JobRecord::new("g", "f.pdf", ".pdf", "");
        job.status = JobStatus::Exported;
        assert!(job.is_done());
        job.status = JobStatus::Failed;
        assert!(job.is_done());
        job.status = JobStatus::Cleaned;
        assert!(!job.is_done());
    }

    #[test]
    fn job_status_display() {
        assert_eq!(format!("{}", JobStatus::Pending), "Pending");
        assert_eq!(format!("{}", JobStatus::Converting), "Converting");
        assert_eq!(format!("{}", JobStatus::Exported), "Exported");
    }

    #[test]
    fn job_serde_roundtrip() {
        let job = JobRecord::new("id-123", "file.docx", ".docx", "");
        let json = serde_json::to_string(&job).unwrap();
        let restored: JobRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.job_guid, "id-123");
        assert_eq!(restored.original_file_name, "file.docx");
        assert_eq!(restored.status, JobStatus::Pending);
    }
}

#[cfg(test)]
mod split_plan_tests {
    use crate::models::split_plan::SplitPlan;

    #[test]
    fn split_plan_single_part() {
        let plan = SplitPlan::new("g", 100, 200);
        assert_eq!(plan.part_count, 1);
        assert_eq!(plan.chapters.len(), 1);
        assert_eq!(plan.chapters[0].start_page, 1);
        assert_eq!(plan.chapters[0].end_page, 100);
    }

    #[test]
    fn split_plan_two_parts() {
        let plan = SplitPlan::new("g", 300, 200);
        assert_eq!(plan.part_count, 2);
        assert_eq!(plan.chapters[0].end_page, 200);
        assert_eq!(plan.chapters[1].start_page, 201);
        assert_eq!(plan.chapters[1].end_page, 300);
    }
}

#[cfg(test)]
mod config_tests {
    use crate::models::config::AppConfig;

    #[test]
    fn default_config_has_sensible_values() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.max_convert_concurrency, 50);
        assert_eq!(cfg.max_clean_concurrency, 3);
        assert_eq!(cfg.pdf_split_threshold_pages, 200);
        assert!(cfg.pdf_split_enabled);
        assert!(!cfg.auto_clean_after_convert);
    }

    #[test]
    fn config_serde_roundtrip() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.max_convert_concurrency, cfg.max_convert_concurrency);
        assert_eq!(restored.openai_base_url, cfg.openai_base_url);
    }
}
