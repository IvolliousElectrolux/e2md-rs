#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum JobStatus {
    Pending,
    Converting,
    Converted,
    Cleaning,
    CleanPartial,
    Cleaned,
    Exported,
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "Pending"),
            JobStatus::Converting => write!(f, "Converting"),
            JobStatus::Converted => write!(f, "Converted"),
            JobStatus::Cleaning => write!(f, "Cleaning"),
            JobStatus::CleanPartial => write!(f, "CleanPartial"),
            JobStatus::Cleaned => write!(f, "Cleaned"),
            JobStatus::Exported => write!(f, "Exported"),
            JobStatus::Failed => write!(f, "Failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct JobRecord {
    pub job_guid: String,
    pub original_file_name: String,
    pub original_file_ext: String,
    pub tmp_path: String,
    pub source_file_path: String,
    pub status: JobStatus,
    pub create_time: String,
    pub selected_rule: Option<String>,
    pub export_path: String,
    pub error_message: Option<String>,
    pub progress: Option<f32>,
}

impl JobRecord {
    pub fn new(guid: &str, original_name: &str, ext: &str, _base_dir: &str) -> Self {
        let tmp_path = format!("tmp/{}", guid);
        let source_file_path = format!("{}/source{}", tmp_path, ext);
        Self {
            job_guid: guid.to_string(),
            original_file_name: original_name.to_string(),
            original_file_ext: ext.to_string(),
            tmp_path,
            source_file_path,
            status: JobStatus::Pending,
            create_time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            selected_rule: None,
            export_path: String::new(),
            error_message: None,
            progress: None,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Converting | JobStatus::Cleaning | JobStatus::CleanPartial
        )
    }

    pub fn is_done(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Exported | JobStatus::Failed
        )
    }
}
