// src/config.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

//! Layered configuration: CLI flags > `ffs.toml` > built-in defaults.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::cli::RunArgs;

pub const CONFIG_FILE_NAME: &str = "ffs.toml";

/// Everything that can be set in `ffs.toml`. All fields optional — an
/// absent `ffs.toml` is not an error, it just means "use CLI/defaults".
#[derive(Debug, Deserialize, Default)]
pub struct FileConfig {
    pub author: Option<String>,
    pub license: Option<String>,
    pub year: Option<i32>,
    pub extensions: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

impl FileConfig {
    /// Load `ffs.toml` from `dir` if it exists. Returns the default (empty)
    /// config if the file is absent.
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let path = dir.join(CONFIG_FILE_NAME);
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
    }
}

/// Fully resolved configuration used for the actual run, after merging
/// CLI flags with `ffs.toml` and built-in defaults.
#[derive(Debug)]
pub struct ResolvedConfig {
    pub author: String,
    pub license: String,
    pub year: i32,
    pub extensions: Vec<String>,
    pub exclude: Vec<String>,
}

const DEFAULT_EXCLUDES: &[&str] = &[
    "target",
    ".git",
    "node_modules",
    "dist",
    "build",
    ".hg",
    ".svn",
];

pub fn current_year() -> i32 {
    chrono::Local::now()
        .format("%Y")
        .to_string()
        .parse()
        .unwrap_or(2026)
}

/// Merge CLI args with an optional file config. CLI wins when present;
/// otherwise fall back to the file, then to a hard-coded default.
pub fn resolve(cli: &RunArgs, file: &FileConfig) -> ResolvedConfig {
    let author = cli
        .author
        .clone()
        .or_else(|| file.author.clone())
        .unwrap_or_else(|| "Erick Adriano".to_string());

    let license = cli
        .license
        .clone()
        .or_else(|| file.license.clone())
        .unwrap_or_else(|| "MIT".to_string());

    let year = cli.year.or(file.year).unwrap_or_else(current_year);

    let extensions = if !cli.extensions.is_empty() {
        cli.extensions.clone()
    } else if let Some(ext) = &file.extensions {
        ext.clone()
    } else {
        vec!["rs".to_string()]
    };

    let mut exclude: Vec<String> = DEFAULT_EXCLUDES.iter().map(|s| s.to_string()).collect();
    if let Some(file_exclude) = &file.exclude {
        exclude = file_exclude.clone();
    }
    if !cli.exclude.is_empty() {
        for e in &cli.exclude {
            if !exclude.contains(e) {
                exclude.push(e.clone());
            }
        }
    }

    ResolvedConfig {
        author,
        license,
        year,
        extensions,
        exclude,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_cli() -> RunArgs {
        RunArgs::default()
    }

    #[test]
    fn falls_back_to_builtin_defaults() {
        let resolved = resolve(&empty_cli(), &FileConfig::default());
        assert_eq!(resolved.author, "Erick Adriano");
        assert_eq!(resolved.license, "MIT");
        assert_eq!(resolved.extensions, vec!["rs".to_string()]);
        assert!(resolved.exclude.contains(&"target".to_string()));
    }

    #[test]
    fn file_config_overrides_builtin_defaults() {
        let file = FileConfig {
            author: Some("Jane Doe".to_string()),
            license: Some("Apache-2.0".to_string()),
            year: Some(2020),
            extensions: Some(vec!["go".to_string()]),
            exclude: Some(vec!["vendor".to_string()]),
        };
        let resolved = resolve(&empty_cli(), &file);
        assert_eq!(resolved.author, "Jane Doe");
        assert_eq!(resolved.license, "Apache-2.0");
        assert_eq!(resolved.year, 2020);
        assert_eq!(resolved.extensions, vec!["go".to_string()]);
        assert_eq!(resolved.exclude, vec!["vendor".to_string()]);
    }

    #[test]
    fn cli_overrides_file_config() {
        let file = FileConfig {
            author: Some("Jane Doe".to_string()),
            ..Default::default()
        };
        let mut cli = empty_cli();
        cli.author = Some("CLI Author".to_string());
        let resolved = resolve(&cli, &file);
        assert_eq!(resolved.author, "CLI Author");
    }
}
