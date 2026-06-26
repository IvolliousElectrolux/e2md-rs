#![allow(dead_code)]
use std::path::{Path, PathBuf};

/// Resolve the directory to reveal in the system file manager.
///
/// `path` may point to a file or directory (or a non-existent path with a file extension).
pub fn resolve_reveal_dir(path: &Path) -> PathBuf {
    if path.is_dir() {
        return path.to_path_buf();
    }
    if path.is_file() || path.extension().is_some() {
        return path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf());
    }
    path.to_path_buf()
}

/// Open `path` in the system file manager.
///
/// Accepts either a file or directory path. When given a file, opens its parent folder
/// (on Windows, the file is selected in Explorer).
pub fn open_in_file_manager(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        if path.is_file() {
            let arg = format!("/select,{}", path.display());
            let _ = std::process::Command::new("explorer").arg(arg).spawn();
            return;
        }
        let dir = resolve_reveal_dir(path);
        let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    }

    #[cfg(target_os = "macos")]
    {
        let dir = resolve_reveal_dir(path);
        let _ = std::process::Command::new("open").arg(dir).spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let dir = resolve_reveal_dir(path);
        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        log::warn!(
            "open_in_file_manager: unsupported platform for {}",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn resolve_reveal_dir_for_file_path() {
        let path = Path::new("/tmp/export/doc.md");
        assert_eq!(resolve_reveal_dir(path), PathBuf::from("/tmp/export"));
    }

    #[test]
    fn resolve_reveal_dir_for_dir_path() {
        let path = Path::new("/tmp/export/doc");
        // Non-existent path without trailing semantics: treat as opaque path
        assert_eq!(resolve_reveal_dir(path), PathBuf::from("/tmp/export/doc"));
    }
}
