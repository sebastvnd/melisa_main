// src/header.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

//! Per-file decision logic: does this file need a header, does it already
//! have one, does it conflict with a different one?

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::comment::{build_header, header_marker, looks_like_comment_block, CommentStyle};
use crate::io_atomic::atomic_write;

/// Outcome of evaluating a single file.
#[derive(Debug, PartialEq, Eq)]
pub enum FileStatus {
    /// Header written (or would be written, in dry-run mode).
    Added,
    /// Header already present and identical — nothing to do.
    AlreadyPresent,
    /// A *different* header-shaped comment already exists at the top of the
    /// file and `--force` was not given.
    Conflict,
    /// File content isn't valid UTF-8 (almost certainly binary) — skipped.
    NotUtf8,
    /// No known comment style for this extension and no `--comment` override.
    UnknownCommentStyle,
}

/// Everything process_file needs to build a header, decoupled from the CLI
/// so it's easy to unit test.
pub struct HeaderSpec<'a> {
    pub author: &'a str,
    pub year: i32,
    pub license: &'a str,
    pub force: bool,
}

/// Split `content` into an optional leading shebang line and the rest of
/// the file. Returns (shebang_including_newline_or_empty, remainder).
pub fn split_shebang(content: &str) -> (&str, &str) {
    if content.starts_with("#!") {
        if let Some(pos) = content.find('\n') {
            return (&content[..=pos], &content[pos + 1..]);
        }
        return (content, "");
    }
    ("", content)
}

/// Remove a leading block of comment lines followed by a blank line — used
/// to replace a conflicting pre-existing header when `--force` is set.
/// Returns `body` unchanged if the leading block doesn't look like a
/// comment-only block.
fn strip_existing_header<'a>(style: CommentStyle, body: &'a str) -> &'a str {
    match style {
        CommentStyle::Line(_) => {
            let mut idx = 0;
            for line in body.split_inclusive('\n') {
                let trimmed = line.trim_end_matches('\n');
                if trimmed.trim().is_empty() {
                    idx += line.len();
                    break;
                }
                if !looks_like_comment_block(style, trimmed) {
                    return body;
                }
                idx += line.len();
            }
            &body[idx..]
        }
        CommentStyle::Block(_, _, close) => {
            if let Some(close_pos) = body.find(close) {
                let mut idx = close_pos + close.len();
                // Also consume the blank line that follows, if present.
                let rest = &body[idx..];
                if let Some(stripped) = rest.strip_prefix('\n') {
                    idx += 1;
                    if let Some(stripped2) = stripped.strip_prefix('\n') {
                        let _ = stripped2;
                        idx += 1;
                    }
                }
                &body[idx..]
            } else {
                body
            }
        }
    }
}

/// The result of a successful evaluation: what changed (for diff/reporting)
/// plus the full new file content (only meaningful when status == Added).
pub struct Evaluation {
    pub status: FileStatus,
    pub removed_block: String,
    pub added_block: String,
    pub new_content: String,
}

/// Decide what should happen to `path`, without touching the filesystem
/// (aside from reading it). Pure function — easy to unit test.
pub fn evaluate(
    original_content: &str,
    relative_path: &str,
    style: CommentStyle,
    spec: &HeaderSpec,
) -> Evaluation {
    let (shebang, body) = split_shebang(original_content);
    let marker = header_marker(style, relative_path);
    let expected_header = build_header(style, relative_path, spec.author, spec.year, spec.license);

    if body.starts_with(&expected_header) {
        return Evaluation {
            status: FileStatus::AlreadyPresent,
            removed_block: String::new(),
            added_block: String::new(),
            new_content: original_content.to_string(),
        };
    }

    if body.starts_with(&marker) && !spec.force {
        return Evaluation {
            status: FileStatus::Conflict,
            removed_block: String::new(),
            added_block: String::new(),
            new_content: original_content.to_string(),
        };
    }

    let (new_body, removed_block) = if body.starts_with(&marker) && spec.force {
        let stripped = strip_existing_header(style, body);
        let removed = body[..body.len() - stripped.len()].to_string();
        (stripped, removed)
    } else {
        (body, String::new())
    };

    let mut new_content = String::with_capacity(shebang.len() + expected_header.len() + new_body.len());
    new_content.push_str(shebang);
    new_content.push_str(&expected_header);
    new_content.push_str(new_body);

    Evaluation {
        status: FileStatus::Added,
        removed_block,
        added_block: expected_header,
        new_content,
    }
}

/// Read `path`, evaluate it, and — unless `dry_run` — write the result back
/// atomically. Returns the evaluation either way, so callers can render a
/// diff even in dry-run mode.
pub fn process_file(
    path: &Path,
    relative_path: &str,
    style: Option<CommentStyle>,
    spec: &HeaderSpec,
    dry_run: bool,
) -> Result<Evaluation> {
    let Some(style) = style else {
        return Ok(Evaluation {
            status: FileStatus::UnknownCommentStyle,
            removed_block: String::new(),
            added_block: String::new(),
            new_content: String::new(),
        });
    };

    let raw = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let original_content = match String::from_utf8(raw) {
        Ok(s) => s,
        Err(_) => {
            return Ok(Evaluation {
                status: FileStatus::NotUtf8,
                removed_block: String::new(),
                added_block: String::new(),
                new_content: String::new(),
            })
        }
    };

    let evaluation = evaluate(&original_content, relative_path, style, spec);

    if evaluation.status == FileStatus::Added && !dry_run {
        atomic_write(path, evaluation.new_content.as_bytes())
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    Ok(evaluation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::style_for_extension;

    fn spec(force: bool) -> HeaderSpec<'static> {
        HeaderSpec {
            author: "Erick Adriano",
            year: 2026,
            license: "MIT",
            force,
        }
    }

    #[test]
    fn adds_header_to_plain_file() {
        let style = style_for_extension("rs").unwrap();
        let eval = evaluate("fn main() {}\n", "main.rs", style, &spec(false));
        assert_eq!(eval.status, FileStatus::Added);
        assert!(eval.new_content.starts_with("// main.rs\n// Copyright (c) 2026 Erick Adriano\n"));
        assert!(eval.new_content.ends_with("fn main() {}\n"));
    }

    #[test]
    fn is_idempotent() {
        let style = style_for_extension("rs").unwrap();
        let first = evaluate("fn main() {}\n", "main.rs", style, &spec(false));
        let second = evaluate(&first.new_content, "main.rs", style, &spec(false));
        assert_eq!(second.status, FileStatus::AlreadyPresent);
    }

    #[test]
    fn detects_conflicting_header() {
        let style = style_for_extension("rs").unwrap();
        let content = "// main.rs\n// Copyright (c) 2020 Someone Else\n\nfn main() {}\n";
        let eval = evaluate(content, "main.rs", style, &spec(false));
        assert_eq!(eval.status, FileStatus::Conflict);
    }

    #[test]
    fn force_overwrites_conflicting_header() {
        let style = style_for_extension("rs").unwrap();
        let content = "// main.rs\n// Copyright (c) 2020 Someone Else\n\nfn main() {}\n";
        let eval = evaluate(content, "main.rs", style, &spec(true));
        assert_eq!(eval.status, FileStatus::Added);
        assert!(eval.new_content.starts_with("// main.rs\n// Copyright (c) 2026 Erick Adriano\n"));
        assert!(!eval.new_content.contains("Someone Else"));
        assert!(eval.new_content.ends_with("fn main() {}\n"));
    }

    #[test]
    fn preserves_shebang() {
        let style = style_for_extension("sh").unwrap();
        let content = "#!/usr/bin/env bash\necho hi\n";
        let eval = evaluate(content, "script.sh", style, &spec(false));
        let mut lines = eval.new_content.lines();
        assert_eq!(lines.next(), Some("#!/usr/bin/env bash"));
        assert_eq!(lines.next(), Some("# script.sh"));
    }

    #[test]
    fn block_style_roundtrip_is_idempotent() {
        let style = style_for_extension("css").unwrap();
        let first = evaluate("body { color: red; }\n", "styles.css", style, &spec(false));
        assert_eq!(first.status, FileStatus::Added);
        let second = evaluate(&first.new_content, "styles.css", style, &spec(false));
        assert_eq!(second.status, FileStatus::AlreadyPresent);
    }
}
