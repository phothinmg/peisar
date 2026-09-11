//! All AST node types — block-level, inline-level, and extension types.
//!
//! ## CommonMark
//!
//! `Heading`, `Paragraph`, `CodeBlock`, `BlockQuote`, `List`, `ThematicBreak`,
//! `HtmlBlock`, `Text`, `Emphasis`, `Code`, `Link`, `Image`, `HardBreak`,
//! `SoftBreak`, `HtmlInline`.
//!
//! ## GitHub Flavored Markdown (GFM)
//!
//! - **Strikethrough** — `~~text~~` → [`Inline::Strikethrough`]
//! - **Tables** — pipe-delimited tables → [`Block::Table`], [`Table`], [`TableRow`], [`TableCell`]
//! - **Task list** — `- [x] item` → [`TaskState`] on a [`ListItem`]
//! - **Autolink** — bare URLs → [`Inline::Link`] with `autolink: true`
//!
//! ## Kramdown-style attributes
//!
//! Any block element can carry a [`KramdownAttributes`] block that was
//! parsed from a `{: #id .class key="value" }` line immediately after the
//! block.  The HTML renderer applies these as `id`, `class`, and arbitrary
//! attributes on the corresponding opening tag.

use super::span::Span;
use serde::Serialize;
use std::fmt;

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

/// A Markdown document — the root of the AST.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Document {
    #[serde(rename = "type")]
    pub node_type: &'static str,
    pub children: Vec<Block>,
    /// Span of the whole document in the source text.
    pub position: Span,
}

// ---------------------------------------------------------------------------
// Block-level nodes
// ---------------------------------------------------------------------------

/// Block-level nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Block {
    /// `# Heading` through `###### Heading`.
    Heading {
        level: u8,
        children: Vec<Inline>,
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<KramdownAttributes>,
        position: Span,
    },
    /// Plain paragraph text.
    Paragraph {
        children: Vec<Inline>,
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<KramdownAttributes>,
        position: Span,
    },
    /// Fenced or indented code block.
    CodeBlock {
        lang: Option<String>,
        code: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<KramdownAttributes>,
        position: Span,
    },
    /// `> quote` lines.
    BlockQuote {
        children: Vec<Block>,
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<KramdownAttributes>,
        position: Span,
    },
    /// Unordered (`-`, `*`, `+`) or ordered (`1.`) list.
    List {
        ordered: bool,
        items: Vec<ListItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<KramdownAttributes>,
        position: Span,
    },
    /// Thematic break (`---`, `***`, `___`).
    ThematicBreak {
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<KramdownAttributes>,
        position: Span,
    },
    /// Blank-line separated HTML block.
    HtmlBlock { html: String, position: Span },
    /// GFM table.
    Table {
        table: Table,
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<KramdownAttributes>,
        position: Span,
    },
}

// ---------------------------------------------------------------------------
// Lists
// ---------------------------------------------------------------------------

/// A single list item (an `<li>`). Contains nested block content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListItem {
    pub children: Vec<Block>,
    /// GFM task-list state: `None` = not a task, `Some` = checked / unchecked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskState>,
    /// Span of the item in the *sub-document* of its enclosing list.
    pub position: Span,
}

/// GFM task-list checkbox state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TaskState {
    /// `[ ]` — unchecked
    Unchecked,
    /// `[x]` / `[X]` — checked
    Checked,
}

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

/// A GFM table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Table {
    pub header: TableRow,
    pub rows: Vec<TableRow>,
    /// Column alignment specifications (one per column).
    pub alignments: Vec<TableCellAlignment>,
}

/// A single table row (header or body).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

/// A single table cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableCell {
    pub children: Vec<Inline>,
}

/// Column alignment for table cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum TableCellAlignment {
    /// `:---` or `---` — default (left)
    #[default]
    Default,
    /// `:---`
    Left,
    /// `:---:`
    Center,
    /// `---:`
    Right,
}

// ---------------------------------------------------------------------------
// Inline-level nodes
// ---------------------------------------------------------------------------

/// Inline-level nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Inline {
    /// Plain text run.
    Text { value: String, position: Span },
    /// `**bold**` or `__bold__` / `*italic*` or `_italic_`.
    Emphasis {
        level: EmphasisLevel,
        children: Vec<Inline>,
        position: Span,
    },
    /// `` `code` ``
    Code { code: String, position: Span },
    /// `[text](url)` — `autolink` is `true` for bare-URL GFM autolinks.
    Link {
        text: Vec<Inline>,
        url: String,
        title: Option<String>,
        autolink: bool,
        position: Span,
    },
    /// `![alt](url)`
    Image {
        alt: String,
        url: String,
        title: Option<String>,
        position: Span,
    },
    /// Hard line break (two trailing spaces or backslash at EOL).
    HardBreak { position: Span },
    /// Soft line break (single newline within a paragraph).
    SoftBreak { position: Span },
    /// Raw inline HTML.
    HtmlInline { html: String, position: Span },
    /// GFM strikethrough: `~~text~~`.
    Strikethrough {
        children: Vec<Inline>,
        position: Span,
    },
}

/// Emphasis strength.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EmphasisLevel {
    /// `*italic*` / `_italic_`
    Italic,
    /// `**bold**` / `__bold__`
    Bold,
}

// ---------------------------------------------------------------------------
// Kramdown-style attributes
// ---------------------------------------------------------------------------

/// Kramdown-style block-level attributes parsed from a `{:...}` line that
/// follows the block.
///
/// Example Markdown:
///
/// ```text
/// # Heading
/// {: #my-id .big .red data-toggle="modal"}
/// ```
///
/// produces `id = Some("my-id")`, `classes = ["big", "red"]`,
/// `attributes = [("data-toggle", "modal")]`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct KramdownAttributes {
    /// The `id` from `{#id}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// CSS classes from `{.class1 .class2}` (order preserved).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub classes: Vec<String>,
    /// Arbitrary key="value" attributes from `{key="value"}`.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub attributes: Vec<(String, String)>,
}

impl KramdownAttributes {
    /// Return `true` if no attribute was set.
    pub fn is_empty(&self) -> bool {
        self.id.is_none() && self.classes.is_empty() && self.attributes.is_empty()
    }

    /// Render as an HTML attribute string (without leading space).
    ///
    /// Example: `id="my-id" class="big red" data-toggle="modal"`
    pub fn to_html_attr_string(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref id) = self.id {
            parts.push(format!("id=\"{}\"", escape_attr(id)));
        }
        if !self.classes.is_empty() {
            parts.push(format!(
                "class=\"{}\"",
                escape_attr(&self.classes.join(" "))
            ));
        }
        for (k, v) in &self.attributes {
            if k == "style" {
                parts.push(format!("style=\"{}\"", escape_attr(v)));
            }
            parts.push(format!("{}=\"{}\"", escape_attr(k), escape_attr(v)));
        }
        parts.join(" ")
    }
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for child in &self.children {
            writeln!(f, "{child:?}")?;
        }
        Ok(())
    }
}
