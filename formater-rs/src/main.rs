// src/main.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

//! ffs — add copyright/license headers to source files.
//!
//! See README.md for full usage. This file just wires the modules
//! together; the interesting logic lives in `header`, `comment`,
//! `scanner`, `config`, and `report`.

mod cli;
mod comment;
mod config;
mod header;
mod init;
mod io_atomic;
mod report;
mod scanner;

use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;

use cli::{Cli, Command, RunArgs};
use comment::{style_for_extension, style_from_override};
use config::{FileConfig, ResolvedConfig};
use header::{process_file, FileStatus, HeaderSpec};
use report::{print_summary, render_diff, Colorizer, Summary};
use scanner::{scan, ScanOptions};

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Some(Command::Init { force }) => init::run(Path::new("."), *force),
        None => run(&cli.run),
    };

    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(2);
    }
}

fn run(args: &RunArgs) -> Result<()> {
    let dry_run = args.dry_run || args.check;

    let root = std::fs::canonicalize(&args.path)
        .with_context(|| format!("root path not found or inaccessible: {}", args.path.display()))?;

    let file_config = FileConfig::load_from_dir(&root)?;
    let resolved: ResolvedConfig = config::resolve(args, &file_config);

    let color = Colorizer::new(args.no_color);

    println!("{}", color.bold("=== ffs: header tool ==="));
    println!(
        "Scanning extensions {:?} under '{}'{}",
        resolved.extensions,
        root.display(),
        if dry_run { "  (dry run)" } else { "" }
    );
    println!();

    let scan_opts = ScanOptions {
        root: &root,
        exclude_dirs: &resolved.exclude,
        respect_gitignore: args.respect_gitignore,
    };
    let all_files = scan(&scan_opts)?;

    let mut summary = Summary::default();

    for path in all_files {
        let ext = match path.extension().and_then(|s| s.to_str()) {
            Some(e) => e,
            None => continue,
        };
        if !resolved.extensions.iter().any(|target| target == ext) {
            continue;
        }

        let relative_path = to_display_path(&path, &root);

        let style = match &args.comment {
            Some(prefix) => Some(style_from_override(prefix)),
            None => style_for_extension(ext),
        };

        let spec = HeaderSpec {
            author: &resolved.author,
            year: resolved.year,
            license: &resolved.license,
            force: args.force,
        };

        match process_file(&path, &relative_path, style, &spec, dry_run) {
            Ok(eval) => {
                report_one(&color, &relative_path, &eval, &mut summary, args, dry_run);
            }
            Err(err) => {
                summary.errors += 1;
                eprintln!("{} {relative_path}: {err:#}", color.red("[ERROR]"));
            }
        }
    }

    print_summary(&color, &summary, dry_run);

    if args.check && summary.files_changed() > 0 {
        eprintln!(
            "\n[CHECK] {} file(s) are missing the expected header.",
            summary.files_changed()
        );
        std::process::exit(1);
    }
    if summary.errors > 0 {
        std::process::exit(2);
    }

    Ok(())
}

fn report_one(
    color: &Colorizer,
    relative_path: &str,
    eval: &header::Evaluation,
    summary: &mut Summary,
    args: &RunArgs,
    dry_run: bool,
) {
    match eval.status {
        FileStatus::Added => {
            summary.added += 1;
            summary.insertions += eval.added_block.lines().count();
            summary.deletions += eval.removed_block.lines().count();
            let verb = if dry_run { "would add" } else { "added" };
            println!(
                "{} {relative_path}",
                color.green(&format!("[{}]", verb.to_uppercase()))
            );
            if args.diff {
                let context_after = eval
                    .new_content
                    .lines()
                    .nth(eval.added_block.lines().count())
                    .map(|s| s.to_string());
                print!(
                    "{}",
                    render_diff(
                        color,
                        relative_path,
                        &eval.removed_block,
                        &eval.added_block,
                        context_after.as_deref()
                    )
                );
            }
        }
        FileStatus::AlreadyPresent => {
            summary.already_present += 1;
            if args.verbose {
                println!("[up to date] {relative_path}");
            }
        }
        FileStatus::Conflict => {
            summary.conflicts += 1;
            println!(
                "{} {relative_path}",
                color.yellow("[CONFLICT] existing different header, use --force")
            );
        }
        FileStatus::NotUtf8 => {
            summary.not_utf8 += 1;
            if args.verbose {
                println!("[skip: binary] {relative_path}");
            }
        }
        FileStatus::UnknownCommentStyle => {
            summary.unknown_style += 1;
            if args.verbose {
                println!(
                    "[skip: unknown comment style, use --comment] {relative_path}"
                );
            }
        }
    }
}

/// Render `path` as a `/`-separated path relative to `root`.
fn to_display_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
