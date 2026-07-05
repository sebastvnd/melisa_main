// src/comment.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

//! Comment-syntax-aware header construction.
//!
//! Different languages comment differently, so a `.py` or `.html` file
//! shouldn't get a C-style `//` header. This module maps file extensions
//! to a [`CommentStyle`] and renders the header text accordingly.

/// How a language's comments are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentStyle {
    /// A single-line comment prefix, e.g. `//`, `#`, `--`.
    Line(&'static str),
    /// A block comment: opening delimiter, per-line prefix (may be empty),
    /// closing delimiter. E.g. `("/*", " *", "*/")` or `("<!--", "", "-->")`.
    Block(&'static str, &'static str, &'static str),
}

const LINE_SLASH: CommentStyle = CommentStyle::Line("//");
const LINE_HASH: CommentStyle = CommentStyle::Line("#");
const LINE_DASH: CommentStyle = CommentStyle::Line("--");
const BLOCK_C: CommentStyle = CommentStyle::Block("/*", " *", "*/");
const BLOCK_XML: CommentStyle = CommentStyle::Block("<!--", "", "-->");

/// Look up the comment style for a file extension (without the leading
/// dot). Returns `None` for unrecognized extensions — the caller should
/// then fall back to `--comment` or skip the file with a warning.
pub fn style_for_extension(ext: &str) -> Option<CommentStyle> {
    Some(match ext {
        "rs" | "go" | "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hh" | "java" | "js" | "jsx"
        | "mjs" | "cjs" | "ts" | "tsx" | "cs" | "kt" | "kts" | "swift" | "scala" | "php"
        | "groovy" | "dart" | "zig" | "proto" => LINE_SLASH,

        "py" | "rb" | "sh" | "bash" | "zsh" | "fish" | "pl" | "pm" | "yaml" | "yml" | "toml"
        | "r" | "R" | "nim" | "cr" | "ps1" | "dockerfile" | "makefile" | "mk" => LINE_HASH,

        "sql" | "lua" | "hs" | "elm" | "ada" | "adb" | "ads" => LINE_DASH,

        "css" | "scss" | "less" => BLOCK_C,

        "html" | "htm" | "xml" | "vue" | "svelte" | "svg" | "md" | "markdown" => BLOCK_XML,

        _ => return None,
    })
}

/// Parse a user-supplied `--comment` override into a style. Currently
/// supports single-line prefixes only (e.g. `"//"`, `"#"`, `";"`).
pub fn style_from_override(prefix: &str) -> CommentStyle {
    CommentStyle::Line(Box::leak(prefix.to_string().into_boxed_str()))
}

/// Build the full header text (including trailing blank line) for a file.
pub fn build_header(style: CommentStyle, relative_path: &str, author: &str, year: i32, license: &str) -> String {
    match style {
        CommentStyle::Line(p) => format!(
            "{p} {relative_path}\n{p} Copyright (c) {year} {author}\n{p} Licensed under the {license} License.\n\n"
        ),
        CommentStyle::Block(open, mid, close) => {
            let sep = if mid.is_empty() { "" } else { " " };
            format!(
                "{open}\n{mid}{sep}{relative_path}\n{mid}{sep}Copyright (c) {year} {author}\n{mid}{sep}Licensed under the {license} License.\n{close}\n\n"
            )
        }
    }
}

/// The line(s) that uniquely identify "some header for this file already
/// exists here", used for idempotency and conflict detection. Returned as
/// the literal prefix that a well-formed header must start with.
pub fn header_marker(style: CommentStyle, relative_path: &str) -> String {
    match style {
        CommentStyle::Line(p) => format!("{p} {relative_path}"),
        CommentStyle::Block(open, mid, close) => {
            let _ = close;
            let sep = if mid.is_empty() { "" } else { " " };
            format!("{open}\n{mid}{sep}{relative_path}")
        }
    }
}

/// True if every line of `block` looks like a comment line in `style`
/// (used to decide whether it's safe to strip a pre-existing header when
/// `--force` is given).
pub fn looks_like_comment_block(style: CommentStyle, block: &str) -> bool {
    match style {
        CommentStyle::Line(p) => block
            .lines()
            .all(|line| line.trim().is_empty() || line.trim_start().starts_with(p)),
        CommentStyle::Block(open, _, close) => {
            block.trim_start().starts_with(open) && block.trim_end().ends_with(close)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_uses_line_slash() {
        assert_eq!(style_for_extension("rs"), Some(LINE_SLASH));
    }

    #[test]
    fn python_uses_hash() {
        assert_eq!(style_for_extension("py"), Some(LINE_HASH));
    }

    #[test]
    fn html_uses_xml_block() {
        assert_eq!(style_for_extension("html"), Some(BLOCK_XML));
    }

    #[test]
    fn unknown_extension_returns_none() {
        assert_eq!(style_for_extension("xyz123"), None);
    }

    #[test]
    fn line_header_format() {
        let header = build_header(LINE_SLASH, "src/main.rs", "Jane", 2026, "MIT");
        assert_eq!(
            header,
            "// src/main.rs\n// Copyright (c) 2026 Jane\n// Licensed under the MIT License.\n\n"
        );
    }

    #[test]
    fn block_header_format() {
        let header = build_header(BLOCK_C, "styles.css", "Jane", 2026, "MIT");
        assert_eq!(
            header,
            "/*\n * styles.css\n * Copyright (c) 2026 Jane\n * Licensed under the MIT License.\n*/\n\n"
        );
    }

    #[test]
    fn xml_header_format() {
        let header = build_header(BLOCK_XML, "index.html", "Jane", 2026, "MIT");
        assert_eq!(
            header,
            "<!--\nindex.html\nCopyright (c) 2026 Jane\nLicensed under the MIT License.\n-->\n\n"
        );
    }

    #[test]
    fn marker_matches_header_prefix() {
        let header = build_header(LINE_SLASH, "src/main.rs", "Jane", 2026, "MIT");
        let marker = header_marker(LINE_SLASH, "src/main.rs");
        assert!(header.starts_with(&marker));
    }
}
