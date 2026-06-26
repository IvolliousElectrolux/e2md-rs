#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterEntry {
    pub title: String,
    pub start_page: u32,
    pub end_page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitPlan {
    pub job_guid: String,
    pub total_pages: u32,
    pub threshold_pages: u32,
    pub chapters: Vec<ChapterEntry>,
    pub part_count: u32,
}

impl SplitPlan {
    pub fn new(guid: &str, total_pages: u32, threshold: u32) -> Self {
        let part_count = (total_pages + threshold - 1) / threshold;
        let mut chapters = Vec::new();
        for i in 0..part_count {
            let start = i * threshold + 1;
            let end = ((i + 1) * threshold).min(total_pages);
            chapters.push(ChapterEntry {
                title: format!("Part {}", i + 1),
                start_page: start,
                end_page: end,
            });
        }
        Self {
            job_guid: guid.to_string(),
            total_pages,
            threshold_pages: threshold,
            chapters,
            part_count,
        }
    }
}
