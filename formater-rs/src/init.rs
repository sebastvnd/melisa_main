// src/init.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

//! `ffs init` — scaffold a `.ffsignore` and `ffs.toml` in the current
//! (or given) directory.

use std::fs;
use std::path::Path;

use anyhow::{bail, Result};

const FFSIGNORE_TEMPLATE: &str = "\
# .ffsignore — gitignore-style rules for files/dirs `ffs` should skip.
# Supports the same syntax as .gitignore, including negation with `!`.
#
# Examples:
# generated/
# *.gen.rs
# vendor/**
# !vendor/keep-this.rs
";

const CONFIG_TEMPLATE: &str = "\
# ffs.toml — default settings for `ffs`. CLI flags always override these.

author = \"Erick Adriano\"
license = \"MIT\"
# year = 2026                # omit to always use the current year
extensions = [\"rs\"]
exclude = [\"target\", \".git\", \"node_modules\", \"dist\", \"build\"]
";

/// Write both scaffold files into `dir`. Refuses to overwrite existing
/// files unless `force` is true.
pub fn run(dir: &Path, force: bool) -> Result<()> {
    write_scaffold(&dir.join(".ffsignore"), FFSIGNORE_TEMPLATE, force)?;
    write_scaffold(&dir.join("ffs.toml"), CONFIG_TEMPLATE, force)?;
    Ok(())
}

fn write_scaffold(path: &Path, contents: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        println!(
            "[SKIPPED] {} already exists (use --force to overwrite)",
            path.display()
        );
        return Ok(());
    }
    fs::write(path, contents)?;
    if !path.exists() {
        bail!("failed to write {}", path.display());
    }
    println!("[CREATED] {}", path.display());
    Ok(())
}
