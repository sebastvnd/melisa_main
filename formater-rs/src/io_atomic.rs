// src/io_atomic.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

//! Atomic file writes: write to a sibling temp file, then rename over the
//! original so a crash mid-write can never corrupt the target file.

use std::fs;
use std::io;
use std::path::Path;

pub fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "tmp".to_string());
    let tmp_path = dir.join(format!(".{file_name}.{}.tmp", std::process::id()));

    {
        use std::io::Write;
        let mut tmp_file = fs::File::create(&tmp_path)?;
        tmp_file.write_all(data)?;
        tmp_file.sync_all()?;
    }

    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn writes_and_replaces_file_contents() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, b"old").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn does_not_leave_temp_file_behind() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        atomic_write(&path, b"content").unwrap();
        let leftover: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftover.is_empty());
    }
}
