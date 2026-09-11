//! Configuration options for parsing and rendering.
//!
//! Use [`ParseOptions`] to control which Markdown extensions are active,
//! and [`RenderOptions`] to control HTML output format (full document vs.
//! fragment).

use serde::{Deserialize, Serialize};

/// Options that control how Markdown is parsed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseOptions {
    /// Enable GitHub Flavored Markdown (tables, strikethrough, task lists,
    /// autolinks).  Default: `true`.
    pub gfm: bool,
    /// Enable Kramdown-style block attributes (`{:#id .class key="val"}`).
    /// Default: `true`.
    pub kramdown: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            gfm: true,
            kramdown: true,
        }
    }
}

/// Options that control how the AST is rendered to HTML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderOptions {
    /// If `true`, emit only the body content (no `<!DOCTYPE>`, `<html>`,
    /// `<head>`, or `<body>` wrapper).  If `false`, emit a full HTML
    /// document.  Default: `true` (fragment).
    pub fragment: bool,
    /// Include a `<meta charset="utf-8">` in the head (only relevant when
    /// `fragment` is `false`).  Default: `true`.
    pub charset: bool,
    /// Include a `<meta name="viewport" content="width=device-width,
    /// initial-scale=1.0">` in the head (only relevant when `fragment` is
    /// `false`).  Default: `true`.
    pub viewport: bool,
    /// Optional `<title>` for the HTML head (only relevant when
    /// `fragment` is `false`).  Default: `None`.
    pub title: Option<String>,
    /// Optional additional CSS classes to add to `<body>` (only relevant
    /// when `fragment` is `false`).  Default: `None`.
    pub body_class: Option<String>,
    /// Optional inline CSS to inject in a `<style>` tag in the head.
    /// Default: `None`.
    pub style: Option<String>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            fragment: true,
            charset: true,
            viewport: true,
            title: None,
            body_class: None,
            style: None,
        }
    }
}

/// Convenience: create `Some(true)` for fragment mode (backward-compatible
/// with the old `render_document(doc, Some(bool))` API).
impl From<bool> for RenderOptions {
    fn from(fragment: bool) -> Self {
        Self {
            fragment,
            ..Default::default()
        }
    }
}
