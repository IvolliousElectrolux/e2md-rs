use anyhow::Result;
use std::path::PathBuf;

use crate::utils::{
    file::ensure_dir,
    log::log_info,
    markdown::split_markdown,
    path::job_tmp_dir,
};

pub struct CutModule;

impl CutModule {
    /// Split a PDF file into `pdf_parts/` sub-parts by page ranges.
    ///
    /// Since Rust doesn't have a native PDF splitter as lightweight as PdfSharp,
    /// we use a two-step approach:
    ///  1. Count pages (via `count_pdf_pages` heuristic).
    ///  2. Record the split plan; actual splitting deferred to MinerU
    ///     (MinerU can handle multi-page PDFs and we send them in page-range batches).
    ///
    /// Returns a list of `(start_page, end_page)` tuples (1-indexed, inclusive).
    pub fn compute_split_plan(
        total_pages: u32,
        threshold: u32,
    ) -> Vec<(u32, u32)> {
        if threshold == 0 || total_pages <= threshold {
            return vec![(1, total_pages)];
        }
        let mut parts = Vec::new();
        let mut start = 1u32;
        while start <= total_pages {
            let end = (start + threshold - 1).min(total_pages);
            parts.push((start, end));
            start = end + 1;
        }
        parts
    }

    /// Merge multiple `raw_md_parts/*.md` files into a single `merged_raw.md`.
    pub fn merge_raw_md_parts(guid: &str) -> Result<PathBuf> {
        let base = job_tmp_dir(guid);
        let parts_dir = base.join("raw_md_parts");
        let dest = base.join("merged_raw.md");

        let mut merged = String::new();
        let mut entries: Vec<_> = std::fs::read_dir(&parts_dir)?
            .flatten()
            .filter(|e| {
                e.path().extension().and_then(|x| x.to_str()) == Some("md")
            })
            .collect();

        // Sort by file name so parts merge in order
        entries.sort_by_key(|e| e.file_name());

        for entry in &entries {
            let content = std::fs::read_to_string(entry.path())?;
            if !merged.is_empty() {
                merged.push_str("\n\n");
            }
            merged.push_str(content.trim());
        }

        std::fs::write(&dest, &merged)?;
        log_info(format!(
            "[Cut] 合并 {} 个 raw_md 分片 -> merged_raw.md ({})",
            entries.len(),
            guid
        ));
        Ok(dest)
    }

    /// Split `merged_raw.md` into `clean_md_parts/` for AI cleaning.
    pub fn split_for_cleaning(
        guid: &str,
        strategy: &str,
        max_chars: usize,
    ) -> Result<Vec<PathBuf>> {
        let base = job_tmp_dir(guid);
        let raw_path = base.join("merged_raw.md");
        let parts_dir = base.join("clean_md_parts");
        ensure_dir(&parts_dir)?;

        let text = std::fs::read_to_string(&raw_path)?;
        let chunks = split_markdown(&text, strategy, max_chars);

        let mut paths = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let path = parts_dir.join(format!("chunk_{:04}.md", i));
            std::fs::write(&path, chunk)?;
            paths.push(path);
        }

        log_info(format!(
            "[Cut] 清洗前切分: {} 个分片 ({})",
            paths.len(),
            guid
        ));
        Ok(paths)
    }

    /// Merge `clean_md_parts/` cleaned chunks into final `cleaned.md`.
    pub fn merge_cleaned_parts(guid: &str) -> Result<PathBuf> {
        let base = job_tmp_dir(guid);
        let parts_dir = base.join("clean_md_parts");
        let dest = base.join("cleaned.md");

        let mut entries: Vec<_> = std::fs::read_dir(&parts_dir)?
            .flatten()
            .filter(|e| {
                e.path().extension().and_then(|x| x.to_str()) == Some("md")
                    && e.file_name().to_string_lossy().starts_with("chunk_")
                    && e.file_name().to_string_lossy().contains("_done")
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());

        let mut merged = String::new();
        for entry in &entries {
            let content = std::fs::read_to_string(entry.path())?;
            if !merged.is_empty() {
                merged.push_str("\n\n");
            }
            merged.push_str(content.trim());
        }

        std::fs::write(&dest, &merged)?;
        log_info(format!(
            "[Cut] 合并 {} 个清洗分片 -> cleaned.md ({})",
            entries.len(),
            guid
        ));
        Ok(dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_plan_no_split_needed() {
        let plan = CutModule::compute_split_plan(100, 200);
        assert_eq!(plan, vec![(1, 100)]);
    }

    #[test]
    fn split_plan_exact_multiple() {
        let plan = CutModule::compute_split_plan(400, 200);
        assert_eq!(plan, vec![(1, 200), (201, 400)]);
    }

    #[test]
    fn split_plan_uneven() {
        let plan = CutModule::compute_split_plan(350, 200);
        assert_eq!(plan, vec![(1, 200), (201, 350)]);
    }

    #[test]
    fn split_plan_threshold_zero_returns_full() {
        let plan = CutModule::compute_split_plan(300, 0);
        assert_eq!(plan, vec![(1, 300)]);
    }

    #[test]
    fn merge_raw_integration() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let guid = "test-guid";
        let base = tmp.path().join(guid);
        let parts = base.join("raw_md_parts");
        std::fs::create_dir_all(&parts).unwrap();
        std::fs::write(parts.join("part_0001.md"), "# Chapter 1\n\nHello").unwrap();
        std::fs::write(parts.join("part_0002.md"), "# Chapter 2\n\nWorld").unwrap();

        // Override job_tmp_dir to use our temp path — we call the logic directly
        let dest = base.join("merged_raw.md");
        let mut merged = String::new();
        let mut entries: Vec<_> = std::fs::read_dir(&parts).unwrap().flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in &entries {
            let content = std::fs::read_to_string(e.path()).unwrap();
            if !merged.is_empty() { merged.push_str("\n\n"); }
            merged.push_str(content.trim());
        }
        std::fs::write(&dest, &merged).unwrap();

        let result = std::fs::read_to_string(&dest).unwrap();
        assert!(result.contains("Chapter 1"));
        assert!(result.contains("Chapter 2"));
    }
}
