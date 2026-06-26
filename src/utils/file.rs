#![allow(dead_code)]
use std::io;
use std::path::{Path, PathBuf};

/// Ensure directory exists (creates recursively).
pub fn ensure_dir(path: &Path) -> io::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Copy a file, creating parent directories as needed.
pub fn copy_file(src: &Path, dst: &Path) -> io::Result<()> {
    if let Some(parent) = dst.parent() {
        ensure_dir(parent)?;
    }
    std::fs::copy(src, dst)?;
    Ok(())
}

/// Copy a directory recursively.
pub fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    ensure_dir(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            copy_file(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Normalize a zip entry name to a safe relative path (handles `\` and rejects traversal).
fn zip_entry_relative_path(name: &str) -> Option<PathBuf> {
    let normalized = name.replace('\\', "/");
    let path = Path::new(&normalized);
    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => relative.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => return None,
        }
    }
    Some(relative)
}

/// Extract a ZIP archive in memory to the given destination directory.
pub fn extract_zip(data: &[u8], dest: &Path) -> anyhow::Result<()> {
    use std::io::Read;
    use zip::ZipArchive;

    ensure_dir(dest)?;
    let cursor = std::io::Cursor::new(data);
    let mut archive = ZipArchive::new(cursor)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let entry_name = file.name().to_string();
        let relative = zip_entry_relative_path(&entry_name)
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or_else(|| anyhow::anyhow!("Invalid zip entry path: {}", entry_name))?;
        let outpath = dest.join(relative);
        if entry_name.ends_with('/') || entry_name.ends_with('\\') {
            ensure_dir(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                ensure_dir(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            std::io::Write::write_all(&mut outfile, &buf)?;
        }
    }
    Ok(())
}

/// Remove a directory recursively, ignoring errors.
pub fn remove_dir(path: &Path) {
    let _ = std::fs::remove_dir_all(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = SimpleFileOptions::default();
            for (name, data) in entries {
                zip.start_file(*name, options).unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap();
        }
        buf
    }

    #[test]
    fn extract_zip_normalizes_backslashes() {
        let tmp = tempfile::tempdir().unwrap();
        let data = build_zip(&[("images\\pic.png", b"png"), ("full.md", b"# hi")]);
        extract_zip(&data, tmp.path()).unwrap();
        assert!(tmp.path().join("images/pic.png").exists());
        assert!(tmp.path().join("full.md").exists());
    }

    #[test]
    fn extract_zip_rejects_parent_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let data = build_zip(&[("../escape.txt", b"bad")]);
        assert!(extract_zip(&data, tmp.path()).is_err());
    }
}
