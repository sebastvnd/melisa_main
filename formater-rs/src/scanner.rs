// src/scanner.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

//! Directory traversal. Supports `.ffsignore` files (gitignore syntax,
//! including `!negation`) plus a simple `--exclude <dir-name>` list, and
//! optionally standard `.gitignore` files.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;

pub const IGNORE_FILE_NAME: &str = ".ffsignore";

pub struct ScanOptions<'a> {
    pub root: &'a Path,
    pub exclude_dirs: &'a [String],
    pub respect_gitignore: bool,
}

/// Return every regular file under `opts.root`, honoring `.ffsignore`
/// files at each directory level and the `--exclude` directory list.
/// `.git` is always pruned regardless of configuration, since scanning
/// VCS internals is never useful and could be unsafe to rewrite.
pub fn scan(opts: &ScanOptions) -> Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(opts.root);
    builder
        .hidden(false)
        .parents(false)
        .git_ignore(opts.respect_gitignore)
        .git_global(opts.respect_gitignore)
        .git_exclude(opts.respect_gitignore)
        .ignore(false)
        .add_custom_ignore_filename(IGNORE_FILE_NAME);

    // Turn the `--exclude` directory-name list into override patterns.
    // In the `ignore` crate, an Override built entirely from `!`-prefixed
    // (negated) patterns acts as a pure blacklist: anything matching one
    // of these is excluded, everything else stays included by default.
    let mut always_excluded = vec![".git".to_string()];
    for name in opts.exclude_dirs {
        if !always_excluded.contains(name) {
            always_excluded.push(name.clone());
        }
    }

    let mut ov = OverrideBuilder::new(opts.root);
    for name in &always_excluded {
        ov.add(&format!("!{name}"))
            .with_context(|| format!("invalid --exclude pattern: {name}"))?;
        ov.add(&format!("!{name}/**"))
            .with_context(|| format!("invalid --exclude pattern: {name}/**"))?;
    }
    let overrides = ov
        .build()
        .context("failed to build directory exclude rules")?;
    builder.overrides(overrides);

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry.context("failed to read a directory entry while scanning")?;
        let is_file = entry.file_type().map(|t| t.is_file()).unwrap_or(false);
        if !is_file {
            continue;
        }
        // These are ffs's own metadata files, never treat them as scan targets.
        if let Some(name) = entry.file_name().to_str() {
            if name == IGNORE_FILE_NAME || name == crate::config::CONFIG_FILE_NAME {
                continue;
            }
        }
        files.push(entry.into_path());
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn finds_plain_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "fn a() {}").unwrap();
        fs::write(dir.path().join("b.rs"), "fn b() {}").unwrap();

        let opts = ScanOptions {
            root: dir.path(),
            exclude_dirs: &[],
            respect_gitignore: false,
        };
        let mut files: Vec<_> = scan(&opts)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        files.sort();
        assert_eq!(files, vec!["a.rs".to_string(), "b.rs".to_string()]);
    }

    #[test]
    fn prunes_excluded_directories() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("target")).unwrap();
        fs::write(dir.path().join("target/ignored.rs"), "// nope").unwrap();
        fs::write(dir.path().join("kept.rs"), "fn kept() {}").unwrap();

        let opts = ScanOptions {
            root: dir.path(),
            exclude_dirs: &["target".to_string()],
            respect_gitignore: false,
        };
        let files = scan(&opts).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "kept.rs");
    }

    #[test]
    fn honors_ffsignore_file() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".ffsignore"), "generated/\n*.gen.rs\n").unwrap();
        fs::create_dir(dir.path().join("generated")).unwrap();
        fs::write(dir.path().join("generated/skip.rs"), "// nope").unwrap();
        fs::write(dir.path().join("keep.gen.rs"), "// nope either").unwrap();
        fs::write(dir.path().join("keep.rs"), "fn keep() {}").unwrap();

        let opts = ScanOptions {
            root: dir.path(),
            exclude_dirs: &[],
            respect_gitignore: false,
        };
        let files: Vec<_> = scan(&opts)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(files, vec!["keep.rs".to_string()]);
    }

    #[test]
    fn ffsignore_negation_works() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".ffsignore"), "*.rs\n!keep.rs\n").unwrap();
        fs::write(dir.path().join("skip.rs"), "// nope").unwrap();
        fs::write(dir.path().join("keep.rs"), "fn keep() {}").unwrap();

        let opts = ScanOptions {
            root: dir.path(),
            exclude_dirs: &[],
            respect_gitignore: false,
        };
        let files: Vec<_> = scan(&opts)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(files, vec!["keep.rs".to_string()]);
    }
}
