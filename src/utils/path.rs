#![allow(dead_code)]
use std::path::{Path, PathBuf};

/// Returns the executable directory (or current dir as fallback).
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `<exe_dir>/tmp`
pub fn tmp_dir() -> PathBuf {
    exe_dir().join("tmp")
}

/// `<exe_dir>/tmp/<guid>`
pub fn job_tmp_dir(guid: &str) -> PathBuf {
    tmp_dir().join(guid)
}

/// `<exe_dir>/tmp/job.json`
pub fn job_json_path() -> PathBuf {
    tmp_dir().join("job.json")
}

/// `<exe_dir>/export`
pub fn export_dir() -> PathBuf {
    exe_dir().join("export")
}

/// `<exe_dir>/export/<stem_name>/`
pub fn job_export_dir(original_name: &str) -> PathBuf {
    let stem = Path::new(original_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(original_name);
    export_dir().join(stem)
}

/// `<exe_dir>/export/<stem>_raw/`  — raw conversion output
pub fn job_export_dir_raw(original_name: &str) -> PathBuf {
    let stem = Path::new(original_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(original_name);
    export_dir().join(format!("{}_raw", stem))
}

/// `<exe_dir>/export/<stem>_<rule_name>/`  — AI cleaned output
///
/// `rule_name` is sanitized: non-alphanumeric chars become `_`.
pub fn job_export_dir_cleaned(original_name: &str, rule_name: &str) -> PathBuf {
    let stem = Path::new(original_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(original_name);
    let safe_rule: String = rule_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    export_dir().join(format!("{}_{}", stem, safe_rule))
}

/// `<exe_dir>/rules`
pub fn rules_dir() -> PathBuf {
    exe_dir().join("rules")
}

/// Sanitize a filename to ASCII-safe characters (for MinerU data_id).
pub fn to_ascii_safe(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Fix image relative paths in markdown so they resolve correctly in export dir.
/// Rewrites `images/foo.png` → `images/foo.png` (keeps relative as-is).
pub fn fix_image_paths(md: &str, _export_subdir: &str) -> String {
    // MinerU outputs paths like `images/xxx.png`; Obsidian reads them relative
    // to the note file, so no change needed as long as we preserve the images/
    // folder alongside the .md file.
    md.to_string()
}

/// Count pages of a PDF by looking at `%%EOF` or stream count heuristic.
/// Returns None if the file cannot be read or parsed.
pub fn count_pdf_pages(path: &Path) -> Option<u32> {
    let data = std::fs::read(path).ok()?;
    // Simple heuristic: count "/Page " occurrences in the cross-reference table
    let text = String::from_utf8_lossy(&data);
    let count = text.matches("/Type /Page\n").count()
        + text.matches("/Type/Page\n").count()
        + text.matches("/Type /Page\r").count()
        + text.matches("/Type/Page\r").count();
    if count > 0 {
        Some(count as u32)
    } else {
        None
    }
}
