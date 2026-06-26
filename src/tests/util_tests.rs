/// Tests for utility modules: markdown, path, file, log, api_usage

#[cfg(test)]
mod markdown_tests {
    use crate::utils::markdown::*;

    #[test]
    fn strip_markdown_fence_with_markdown_tag() {
        let input = "```markdown\n# Title\n\nBody text\n```";
        let result = strip_markdown_fence(input);
        assert_eq!(result, "# Title\n\nBody text");
    }

    #[test]
    fn strip_markdown_fence_without_tag() {
        let input = "```\n# Title\n```";
        let result = strip_markdown_fence(input);
        assert_eq!(result, "# Title");
    }

    #[test]
    fn strip_markdown_fence_no_fence() {
        let input = "# Title\n\nPlain content";
        let result = strip_markdown_fence(input);
        assert_eq!(result, "# Title\n\nPlain content");
    }

    #[test]
    fn strip_markdown_fence_trims_whitespace() {
        let input = "  \n```markdown\nContent\n```\n  ";
        let result = strip_markdown_fence(input);
        assert_eq!(result, "Content");
    }

    #[test]
    fn split_by_chars_single_chunk_when_zero() {
        let text = "hello world";
        let chunks = split_markdown(text, "MaxTokens", 0);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "hello world");
    }

    #[test]
    fn split_by_chars_exactly_one_chunk() {
        let text = "abc";
        let chunks = split_markdown(text, "MaxTokens", 10);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn split_by_chars_multiple_chunks() {
        let text = "1234567890";
        let chunks = split_markdown(text, "MaxTokens", 3);
        assert_eq!(chunks, vec!["123", "456", "789", "0"]);
    }

    #[test]
    fn split_by_heading_empty_text() {
        let text = "";
        let chunks = split_markdown(text, "MarkdownHeading", 0);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "");
    }

    #[test]
    fn split_by_heading_single_chapter() {
        let text = "# Only Chapter\n\nSome content here.";
        let chunks = split_markdown(text, "MarkdownHeading", 0);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].contains("Only Chapter"));
    }

    #[test]
    fn split_by_heading_multiple_h2() {
        let text = "## Section A\n\nContent A\n## Section B\n\nContent B\n## Section C\n\nContent C";
        let chunks = split_markdown(text, "MarkdownHeading", 0);
        assert_eq!(chunks.len(), 3);
        assert!(chunks[0].contains("Section A"));
        assert!(chunks[1].contains("Section B"));
        assert!(chunks[2].contains("Section C"));
    }

    #[test]
    fn chunk_hash_is_stable() {
        let h1 = chunk_hash("test content");
        let h2 = chunk_hash("test content");
        assert_eq!(h1, h2);
    }

    #[test]
    fn chunk_hash_differs_for_different_input() {
        let h1 = chunk_hash("content A");
        let h2 = chunk_hash("content B");
        assert_ne!(h1, h2);
    }

    #[test]
    fn chunk_hash_is_hex_string() {
        let h = chunk_hash("hello");
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(h.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn pre_clean_collapses_excess_blank_lines() {
        let text = "line1\n\n\n\n\n\nline2";
        let result = pre_clean(text);
        assert!(!result.contains("\n\n\n"));
    }

    #[test]
    fn pre_clean_preserves_double_blank() {
        let text = "para1\n\n\npara2";
        let result = pre_clean(text);
        assert!(result.contains("para1"));
        assert!(result.contains("para2"));
    }
}

#[cfg(test)]
mod path_tests {
    use crate::utils::path::*;

    #[test]
    fn to_ascii_safe_chinese_becomes_underscores() {
        let result = to_ascii_safe("高等数学.pdf");
        assert!(!result.contains("高"));
        assert!(!result.contains("等"));
        assert!(!result.contains("数"));
        assert!(result.ends_with(".pdf") || result.contains("_"));
    }

    #[test]
    fn to_ascii_safe_keeps_valid_chars() {
        let result = to_ascii_safe("my-file_v1.2.pdf");
        assert_eq!(result, "my-file_v1.2.pdf");
    }

    #[test]
    fn to_ascii_safe_spaces_become_underscores() {
        let result = to_ascii_safe("my file.pdf");
        assert_eq!(result, "my_file.pdf");
    }
}

#[cfg(test)]
mod log_tests {
    use crate::utils::log::*;

    #[test]
    fn log_entry_format_contains_message() {
        let entry = LogEntry::info("Hello world");
        let formatted = entry.format();
        assert!(formatted.contains("Hello world"));
    }

    #[test]
    fn log_entry_levels() {
        assert_eq!(LogEntry::info("x").level, LogLevel::Info);
        assert_eq!(LogEntry::warn("x").level, LogLevel::Warn);
        assert_eq!(LogEntry::error("x").level, LogLevel::Error);
        assert_eq!(LogEntry::success("x").level, LogLevel::Success);
    }

}

#[cfg(test)]
mod api_usage_tests {
    use crate::utils::api_usage::*;

    #[test]
    fn record_and_get_usage() {
        reset_usage();
        record_usage("openrouter", 1000, 0.005);
        record_usage("deepseek", 500, 0.0);
        let all = get_usage();
        assert_eq!(all.openrouter.tokens, 1000);
        assert_eq!(all.deepseek.tokens, 500);
        assert_eq!(all.openai.tokens, 0);
    }

    #[test]
    fn reset_clears_all() {
        record_usage("openai", 9999, 1.0);
        reset_usage();
        let all = get_usage();
        assert_eq!(all.openai.tokens, 0);
        assert_eq!(all.openrouter.tokens, 0);
    }
}
