// mcore/mlog/rotation.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use chrono::Local;
/// Log rotation dan file management
use std::fs;
use std::path::PathBuf;

pub struct LogRotator {
    base_path: PathBuf,
    max_file_size_bytes: u64,
    max_backups: usize,
}

impl LogRotator {
    pub fn new(base_path: PathBuf, max_size_mb: u64, max_backups: usize) -> Self {
        LogRotator {
            base_path,
            max_file_size_bytes: max_size_mb * 1024 * 1024,
            max_backups,
        }
    }

    /// Check dan lakukan rotation jika diperlukan
    pub fn check_and_rotate(&self) -> std::io::Result<()> {
        if !self.base_path.exists() {
            return Ok(());
        }

        let metadata = fs::metadata(&self.base_path)?;
        if metadata.len() >= self.max_file_size_bytes {
            self.rotate()?;
        }

        Ok(())
    }

    /// Rotate log file dengan timestamp
    fn rotate(&self) -> std::io::Result<()> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let file_name = self
            .base_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("app");

        let backup_name = format!("{}.{}", file_name, timestamp);
        let backup_path = self.base_path.parent().map(|p| p.join(&backup_name));

        if let Some(bp) = backup_path {
            fs::rename(&self.base_path, bp)?;
        }

        // Cleanup old backups
        self.cleanup_old_backups()?;

        Ok(())
    }

    /// Hapus backup yang terlalu tua
    fn cleanup_old_backups(&self) -> std::io::Result<()> {
        let parent = match self.base_path.parent() {
            Some(p) => p,
            None => return Ok(()),
        };

        let file_name = match self.base_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => return Ok(()),
        };

        let mut backups: Vec<PathBuf> = fs::read_dir(parent)?
            .filter_map(|e| {
                e.ok().and_then(|entry| {
                    let path = entry.path();
                    let name = path.file_name()?.to_str()?.to_string();
                    if name.starts_with(file_name) && name.len() > file_name.len() {
                        Some(path)
                    } else {
                        None
                    }
                })
            })
            .collect();

        if backups.len() > self.max_backups {
            backups.sort_by_key(|p| fs::metadata(p).ok().and_then(|m| m.modified().ok()));

            // Delete oldest backups
            for old_backup in backups.iter().take(backups.len() - self.max_backups) {
                let _ = fs::remove_file(old_backup);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_log_rotator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let rotator = LogRotator::new(log_path.clone(), 1, 5);
        assert_eq!(rotator.max_file_size_bytes, 1024 * 1024);
        assert_eq!(rotator.max_backups, 5);
    }
}
