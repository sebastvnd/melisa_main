// src/report.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

//! Git-style output: colored per-file status lines, an optional unified
//! diff for each change, and a final summary line.

use std::io::IsTerminal;

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

#[derive(Clone, Copy)]
pub struct Colorizer {
    enabled: bool,
}

impl Colorizer {
    pub fn new(no_color_flag: bool) -> Self {
        let no_color_env = std::env::var_os("NO_COLOR").is_some();
        let is_tty = std::io::stdout().is_terminal();
        Colorizer {
            enabled: !no_color_flag && !no_color_env && is_tty,
        }
    }

    fn wrap(&self, code: &str, text: &str) -> String {
        if self.enabled {
            format!("{code}{text}{RESET}")
        } else {
            text.to_string()
        }
    }

    pub fn green(&self, text: &str) -> String {
        self.wrap(GREEN, text)
    }
    pub fn red(&self, text: &str) -> String {
        self.wrap(RED, text)
    }
    pub fn cyan(&self, text: &str) -> String {
        self.wrap(CYAN, text)
    }
    pub fn yellow(&self, text: &str) -> String {
        self.wrap(YELLOW, text)
    }
    pub fn bold(&self, text: &str) -> String {
        self.wrap(BOLD, text)
    }
}

/// Render a `git diff`-style unified hunk for a header change. Since we
/// always know exactly which lines were removed/added (no generic diff
/// algorithm needed), this is a direct, honest rendering rather than a
/// heuristic one.
pub fn render_diff(
    c: &Colorizer,
    relative_path: &str,
    removed_block: &str,
    added_block: &str,
    context_after: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str(&c.bold(&format!("diff --ffs a/{relative_path} b/{relative_path}\n")));
    out.push_str(&c.bold(&format!("--- a/{relative_path}\n")));
    out.push_str(&c.bold(&format!("+++ b/{relative_path}\n")));

    let removed_lines: Vec<&str> = if removed_block.is_empty() {
        vec![]
    } else {
        removed_block.lines().collect()
    };
    let added_lines: Vec<&str> = added_block.lines().collect();

    let context_count = if context_after.is_some() { 1 } else { 0 };
    let old_count = removed_lines.len() + context_count;
    let new_count = added_lines.len() + context_count;

    out.push_str(&c.cyan(&format!("@@ -1,{old_count} +1,{new_count} @@\n")));

    for line in &removed_lines {
        out.push_str(&c.red(&format!("-{line}\n")));
    }
    for line in &added_lines {
        out.push_str(&c.green(&format!("+{line}\n")));
    }
    if let Some(ctx) = context_after {
        out.push_str(&format!(" {ctx}\n"));
    }

    out
}

#[derive(Debug, Default)]
pub struct Summary {
    pub added: usize,
    pub already_present: usize,
    pub conflicts: usize,
    pub not_utf8: usize,
    pub unknown_style: usize,
    pub errors: usize,
    pub insertions: usize,
    pub deletions: usize,
}

impl Summary {
    pub fn files_changed(&self) -> usize {
        self.added
    }
}

pub fn print_summary(c: &Colorizer, summary: &Summary, dry_run: bool) {
    println!();
    let verb = if dry_run { "Would change" } else { "Changed" };
    println!(
        "{} {} file(s), {} insertion(s)(+), {} deletion(s)(-)",
        verb,
        summary.files_changed(),
        summary.insertions,
        summary.deletions
    );
    if summary.already_present > 0 {
        println!("Already up to date: {}", summary.already_present);
    }
    if summary.conflicts > 0 {
        println!(
            "{}",
            c.yellow(&format!(
                "Conflicts (pre-existing different header, use --force): {}",
                summary.conflicts
            ))
        );
    }
    if summary.unknown_style > 0 {
        println!(
            "{}",
            c.yellow(&format!(
                "Skipped, unknown comment style: {}",
                summary.unknown_style
            ))
        );
    }
    if summary.not_utf8 > 0 {
        println!("Skipped, not valid UTF-8: {}", summary.not_utf8);
    }
    if summary.errors > 0 {
        println!("{}", c.red(&format!("Errors: {}", summary.errors)));
    }
}
