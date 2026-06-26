#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueuePoolType {
    Convert,
    Clean,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueItemStatus {
    Waiting,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl std::fmt::Display for QueueItemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueItemStatus::Waiting => write!(f, "等待中"),
            QueueItemStatus::Running => write!(f, "运行中"),
            QueueItemStatus::Done => write!(f, "完成"),
            QueueItemStatus::Failed => write!(f, "失败"),
            QueueItemStatus::Cancelled => write!(f, "已取消"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub job_guid: String,
    pub file_name: String,
    pub pool_type: QueuePoolType,
    pub status: QueueItemStatus,
    pub priority: i32,
    pub progress: f32,
    pub current_stage: Option<String>,
    pub created_at: std::time::SystemTime,
}

impl QueueItem {
    pub fn new(job_guid: &str, file_name: &str, pool_type: QueuePoolType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            job_guid: job_guid.to_string(),
            file_name: file_name.to_string(),
            pool_type,
            status: QueueItemStatus::Waiting,
            priority: 0,
            progress: 0.0,
            current_stage: None,
            created_at: std::time::SystemTime::now(),
        }
    }
}
