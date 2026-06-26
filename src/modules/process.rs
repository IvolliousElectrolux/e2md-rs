use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{JobRecord, JobStatus};
use crate::utils::{
    file::{copy_dir, copy_file, ensure_dir, remove_dir},
    log::{log_info, log_success},
    path::{job_export_dir, job_json_path, job_tmp_dir, tmp_dir},
};

pub type SharedJobs = Arc<Mutex<Vec<JobRecord>>>;

pub struct ProcessModule {
    jobs: SharedJobs,
}

impl ProcessModule {
    /// Initialize: load `tmp/job.json`, clean zombie tasks.
    pub fn init() -> Result<Self> {
        ensure_dir(&tmp_dir())?;
        let jobs = Self::load_jobs()?;

        // Mark any "active" jobs at startup as Failed (zombie cleanup)
        let mut cleaned = jobs.clone();
        let mut changed = false;
        for job in &mut cleaned {
            if job.is_active() {
                log_info(format!(
                    "清理僵尸任务: {} ({})",
                    job.original_file_name, job.job_guid
                ));
                job.status = JobStatus::Failed;
                job.error_message = Some("应用上次异常退出, 任务状态已重置".to_string());
                changed = true;
            }
        }
        if changed {
            Self::save_jobs_inner(&cleaned)?;
        }

        Ok(Self {
            jobs: Arc::new(Mutex::new(cleaned)),
        })
    }

    pub fn jobs(&self) -> SharedJobs {
        Arc::clone(&self.jobs)
    }

    /// Create a new job from a source file path.
    pub fn create_job(&self, source_path: &std::path::Path) -> Result<JobRecord> {
        let guid = Uuid::new_v4().to_string();
        let file_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let ext = source_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();

        let job_dir = job_tmp_dir(&guid);
        ensure_dir(&job_dir).context("Creating job tmp dir")?;

        let dest = job_dir.join(format!("source{}", ext));
        copy_file(source_path, &dest).context("Copying source file")?;

        let record = JobRecord::new(&guid, &file_name, &ext, "");

        {
            let mut jobs = self.jobs.lock();
            jobs.push(record.clone());
            Self::save_jobs_inner(&jobs)?;
        }

        log_info(format!("已创建任务: {} ({})", file_name, guid));
        Ok(record)
    }

    pub fn update_status(&self, guid: &str, status: JobStatus) -> Result<()> {
        let mut jobs = self.jobs.lock();
        if let Some(job) = jobs.iter_mut().find(|j| j.job_guid == guid) {
            job.status = status;
        }
        Self::save_jobs_inner(&jobs)
    }

    pub fn update_progress(&self, guid: &str, progress: f32) -> Result<()> {
        let mut jobs = self.jobs.lock();
        if let Some(job) = jobs.iter_mut().find(|j| j.job_guid == guid) {
            job.progress = Some(progress.clamp(0.0, 1.0));
        }
        Self::save_jobs_inner(&jobs)
    }

    pub fn set_error(&self, guid: &str, msg: &str) -> Result<()> {
        let mut jobs = self.jobs.lock();
        if let Some(job) = jobs.iter_mut().find(|j| j.job_guid == guid) {
            job.status = JobStatus::Failed;
            job.error_message = Some(msg.to_string());
        }
        Self::save_jobs_inner(&jobs)
    }

    pub fn set_rule(&self, guid: &str, rule: &str) -> Result<()> {
        let mut jobs = self.jobs.lock();
        if let Some(job) = jobs.iter_mut().find(|j| j.job_guid == guid) {
            job.selected_rule = Some(rule.to_string());
        }
        Self::save_jobs_inner(&jobs)
    }

    /// Export a cleaned/converted job to the export directory.
    ///
    /// Copies `merged_raw.md` (or `cleaned.md` if available) and `images/` folder.
    pub fn export(&self, guid: &str) -> Result<()> {
        let record = {
            let jobs = self.jobs.lock();
            jobs.iter()
                .find(|j| j.job_guid == guid)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Job {} not found", guid))?
        };

        let tmp_dir = job_tmp_dir(guid);
        let export_subdir = job_export_dir(&record.original_file_name);
        ensure_dir(&export_subdir)?;

        // Prefer cleaned.md, then merged_raw.md
        let md_source = {
            let cleaned = tmp_dir.join("cleaned.md");
            let raw = tmp_dir.join("merged_raw.md");
            if cleaned.exists() { cleaned } else { raw }
        };

        if !md_source.exists() {
            anyhow::bail!("找不到要导出的 Markdown 文件 ({})", guid);
        }

        let stem = std::path::Path::new(&record.original_file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let md_dest = export_subdir.join(format!("{}.md", stem));
        copy_file(&md_source, &md_dest)?;

        // Copy images/ if present
        let images_src = tmp_dir.join("images");
        if images_src.exists() {
            copy_dir(&images_src, &export_subdir.join("images"))?;
        }

        {
            let mut jobs = self.jobs.lock();
            if let Some(job) = jobs.iter_mut().find(|j| j.job_guid == guid) {
                job.status = JobStatus::Exported;
                job.export_path = export_subdir.to_string_lossy().to_string();
            }
            Self::save_jobs_inner(&jobs)?;
        }

        log_success(format!(
            "导出完成: {} -> {}",
            record.original_file_name,
            export_subdir.display()
        ));
        Ok(())
    }

    /// Delete tmp directory and remove job from job.json.
    pub fn cleanup(&self, guid: &str) -> Result<()> {
        remove_dir(&job_tmp_dir(guid));
        let mut jobs = self.jobs.lock();
        jobs.retain(|j| j.job_guid != guid);
        Self::save_jobs_inner(&jobs)?;
        log_info(format!("任务已清理: {}", guid));
        Ok(())
    }

    pub fn get_jobs(&self) -> Vec<JobRecord> {
        self.jobs.lock().clone()
    }

    fn load_jobs() -> Result<Vec<JobRecord>> {
        let path = job_json_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = std::fs::read_to_string(&path)?;
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }
        Ok(serde_json::from_str(&text)?)
    }

    fn save_jobs_inner(jobs: &[JobRecord]) -> Result<()> {
        let path = job_json_path();
        if let Some(parent) = path.parent() {
            ensure_dir(parent)?;
        }
        let text = serde_json::to_string_pretty(jobs)?;
        std::fs::write(&path, text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn with_tmp_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn job_record_new_sets_fields() {
        let record = JobRecord::new("guid-123", "test.pdf", ".pdf", "");
        assert_eq!(record.job_guid, "guid-123");
        assert_eq!(record.original_file_name, "test.pdf");
        assert_eq!(record.status, JobStatus::Pending);
    }
}
