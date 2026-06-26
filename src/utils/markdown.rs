#![allow(dead_code)]
use sha2::{Digest, Sha256};

/// Strip a surrounding ```markdown ... ``` wrapper that LLMs sometimes emit.
pub fn strip_markdown_fence(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(inner) = trimmed.strip_prefix("```markdown") {
        if let Some(core) = inner.strip_suffix("```") {
            return core.trim().to_string();
        }
    }
    if let Some(inner) = trimmed.strip_prefix("```") {
        if let Some(core) = inner.strip_suffix("```") {
            return core.trim().to_string();
        }
    }
    trimmed.to_string()
}

/// Compute a SHA-256 hex digest of a string slice (used for chunk-level caching).
pub fn chunk_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Split markdown into chunks by heading boundaries or approximate character count.
///
/// * `strategy`: `"MarkdownHeading"` splits at `## ` headings.
///   Any other value splits by `max_chars`.
pub fn split_markdown(text: &str, strategy: &str, max_chars: usize) -> Vec<String> {
    if strategy == "MarkdownHeading" {
        split_by_heading(text)
    } else {
        split_by_chars(text, max_chars)
    }
}

fn split_by_heading(text: &str) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if (line.starts_with("## ") || line.starts_with("# ")) && !current.is_empty() {
            chunks.push(current.trim().to_string());
            current = String::new();
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    if chunks.is_empty() {
        chunks.push(text.trim().to_string());
    }
    chunks
}

fn split_by_chars(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = text.chars().collect();
    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk);
        start = end;
    }
    chunks
}

/// Pre-clean markdown: collapse excess blank lines, fix obvious artefacts.
pub fn pre_clean(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut blank_count = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_markdown_fence_removes_wrapper() {
        let input = "```markdown\n# Title\n\nContent here\n```";
        assert_eq!(strip_markdown_fence(input), "# Title\n\nContent here");
    }

    #[test]
    fn strip_markdown_fence_plain_passthrough() {
        let input = "# Title\n\nNo fence here";
        assert_eq!(strip_markdown_fence(input), "# Title\n\nNo fence here");
    }

    #[test]
    fn split_by_chars_basic() {
        let text = "abcde";
        let chunks = split_by_chars(text, 2);
        assert_eq!(chunks, vec!["ab", "cd", "e"]);
    }

    #[test]
    fn split_by_heading_basic() {
        let text = "# Intro\n\nHello\n## Chapter\n\nWorld";
        let chunks = split_by_heading(text);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].contains("Intro"));
        assert!(chunks[1].contains("Chapter"));
    }

    #[test]
    fn chunk_hash_deterministic() {
        let h1 = chunk_hash("hello world");
        let h2 = chunk_hash("hello world");
        assert_eq!(h1, h2);
        assert_ne!(h1, chunk_hash("different"));
    }

    #[test]
    fn pre_clean_collapses_blanks() {
        let text = "line1\n\n\n\n\nline2";
        let cleaned = pre_clean(text);
        // Should have at most 2 blank lines in a row
        assert!(!cleaned.contains("\n\n\n\n"));
    }
}
