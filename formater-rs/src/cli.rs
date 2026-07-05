// src/cli.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

//! Command-line interface definition.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Add a copyright/license header to every matching source file in a
/// directory tree.
#[derive(Parser, Debug)]
#[command(name = "ffs", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub run: RunArgs,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Scaffold a `.ffsignore` and `ffs.toml` in the current directory
    Init {
        /// Overwrite files if they already exist
        #[arg(long)]
        force: bool,
    },
}

/// Arguments for the default (header-insertion) run. Any value left as
/// `None` / empty falls back to `ffs.toml`, and finally to a built-in
/// default — see `config::resolve`.
#[derive(Parser, Debug, Default)]
pub struct RunArgs {
    /// Root directory to scan
    #[arg(short = 'p', long = "path", default_value = ".")]
    pub path: PathBuf,

    /// File extensions to target (without the dot). Repeatable: -e rs -e go
    #[arg(short = 'e', long = "ext")]
    pub extensions: Vec<String>,

    /// Author / copyright holder name
    #[arg(short = 'a', long)]
    pub author: Option<String>,

    /// Copyright year. Defaults to the current year.
    #[arg(short = 'y', long)]
    pub year: Option<i32>,

    /// License name to reference in the header text
    #[arg(short = 'l', long)]
    pub license: Option<String>,

    /// Directory names to prune from the scan, in addition to `.ffsignore`
    /// rules (matched anywhere in the path)
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Force a specific comment style instead of guessing from the file
    /// extension, e.g. "//" or "#"
    #[arg(long)]
    pub comment: Option<String>,

    /// Show what would change, but don't write anything
    #[arg(long)]
    pub dry_run: bool,

    /// CI mode: implies --dry-run and exits non-zero if any file needs a header
    #[arg(long)]
    pub check: bool,

    /// Overwrite an existing (different) header instead of skipping it
    #[arg(long)]
    pub force: bool,

    /// Print a line for every file visited, not just files that changed
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Show a unified, git-style colored diff for every change
    #[arg(long)]
    pub diff: bool,

    /// Disable colored output
    #[arg(long)]
    pub no_color: bool,

    /// Also respect any `.gitignore` files found while walking (off by default)
    #[arg(long)]
    pub respect_gitignore: bool,
}
