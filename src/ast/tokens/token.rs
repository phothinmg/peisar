//! AST node types: blocks, inlines, tables, lists, and link references.
//!
//! This module defines the two principal enums — [`Block`] and [`Inline`] —
//! that together make up the Markdown AST.  Supporting structs such as
//! [`Table`], [`ListItem`], [`LinkReferenceDefinition`], and
//! [`EmphasisLevel`] live here as well.
//!
//! All types implement [`serde::Serialize`] for JSON output and carry
//! `#[cfg_attr(feature = "napi", napi)]` annotations for napi-rs
//! compatibility.

use super::attrs::Attributes;
use super::span::Span;

use napi_derive::napi;
use serde::Serialize;
// ---------------------------------------------------------------------------
// Block-level nodes
// ---------------------------------------------------------------------------

/// Block-level nodes.
#[napi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    /// `# Heading` through `###### Heading`.
    Heading {
        /// Heading level (1–6).
        level: u8,
        /// Inline content of the heading.
        children: Vec<Inline>,
        /// Source span.
        pos: Span,
        /// Optional Kramdown attributes.
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<Attributes>,
    },
    /// Plain paragraph text.
    Paragraph {
        /// Inline content of the paragraph.
        children: Vec<Inline>,
        /// Source span.
        pos: Span,
        /// Optional Kramdown attributes.
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<Attributes>,
    },
    /// Fenced or indented code block.
    CodeBlock {
        /// Language hint from the info string (e.g. `rust` from ` ```rust `).
        lang: Option<String>,
        /// The raw code content.
        code: String,
        /// Source span.
        pos: Span,
        /// Optional Kramdown attributes.
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<Attributes>,
    },
    /// A block quote (`> ...`).  Children are re-parsed as a sub-document.
    BlockQuote {
        /// Nested block content inside the quote.
        children: Vec<Block>,
        /// Source span.
        pos: Span,
        /// Optional Kramdown attributes.
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<Attributes>,
    },
    /// An ordered (`1.`) or unordered (`-`, `*`, `+`) list.
    List {
        /// `true` for ordered lists, `false` for unordered.
        ordered: bool,
        /// The list items.
        items: Vec<ListItem>,
        /// Source span.
        pos: Span,
        /// Optional Kramdown attributes.
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<Attributes>,
    },
    /// A thematic break (`---`, `***`, or `___` on a line by itself).
    ThematicBreak {
        /// Source span.
        pos: Span,
    },
    /// A raw HTML block.
    HtmlBlock {
        /// The raw HTML content.
        html: String,
        /// Source span.
        pos: Span,
        /// Optional Kramdown attributes.
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<Attributes>,
    },
    /// A GFM table.
    Table {
        /// The table structure (header, rows, alignments).
        table: Table,
        /// Source span.
        pos: Span,
        /// Optional Kramdown attributes.
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<Attributes>,
    },
    /// A Markdown link reference definition: `[label]: url "title"`.
    LinkReferenceDefinition {
        /// Normalised label (lowercased, trimmed).
        label: String,
        /// The destination URL.
        url: String,
        /// Optional link title.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Source span.
        pos: Span,
    },
    /// An HTML comment block (`<!-- ... -->`).
    Comment {
        /// The comment text (markers stripped).
        value: String,
        /// Source span.
        pos: Span,
    },
}

// ---------------------------------------------------------------------------
// Link Reference Definitions
// ---------------------------------------------------------------------------

/// A link reference definition collected at the document level.
#[napi(object)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkReferenceDefinition {
    /// Normalised label (lowercased, trimmed).
    pub label: String,
    /// The destination URL.
    pub url: String,
    /// Optional link title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Source span.
    pub pos: Span,
}

// ---------------------------------------------------------------------------
// Lists
// ---------------------------------------------------------------------------

/// A single list item (an `<li>`). Contains nested block content.
#[napi(object)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListItem {
    /// Nested block content of the item.
    pub children: Vec<Block>,
    /// GFM task-list state: `None` = not a task, `Some` = checked / unchecked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskState>,
    /// Span of the item in the *sub-document* of its enclosing list.
    pub pos: Span,
}

/// GFM task-list checkbox state.
#[napi]
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
#[napi(object)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Table {
    /// The header row.
    pub header: TableRow,
    /// Body rows.
    pub rows: Vec<TableRow>,
    /// Column alignment specifications (one per column).
    pub alignments: Vec<TableCellAlignment>,
}

/// A single table row (header or body).
#[napi(object)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableRow {
    /// The cells in this row.
    pub cells: Vec<TableCell>,
}

/// A single table cell.
#[napi(object)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableCell {
    /// Inline content of the cell.
    pub children: Vec<Inline>,
}

/// Column alignment for table cells.
#[napi]
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
#[napi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Inline {
    /// Plain text content.
    Text {
        /// The text value.
        value: String,
        /// Source span.
        pos: Span,
    },
    /// Emphasis (`*italic*` / `**bold**` / `_italic_` / `__bold__`).
    Emphasis {
        /// Emphasis level (italic or bold).
        level: EmphasisLevel,
        /// Nested inline content.
        children: Vec<Inline>,
        /// Source span.
        pos: Span,
    },
    /// Inline code (`` `code` ``).
    Code {
        /// The code text.
        code: String,
        /// Source span.
        pos: Span,
    },
    /// Raw inline HTML (e.g. `<span>`).
    HtmlInline {
        /// The raw HTML string.
        html: String,
        /// Source span.
        pos: Span,
    },
    /// GFM strikethrough: `~~text~~`.
    Strikethrough {
        /// Nested inline content.
        children: Vec<Inline>,
        /// Source span.
        pos: Span,
    },
    /// A hard line break (two trailing spaces or backslash before newline).
    HardBreak {
        /// Source span.
        pos: Span,
    },
    /// A soft line break (ordinary newline within a paragraph).
    SoftBreak {
        /// Source span.
        pos: Span,
    },
    /// An inline image: `![alt](url)`.
    Image {
        /// Alternative text.
        alt: String,
        /// Image URL.
        url: String,
        /// Optional title.
        title: Option<String>,
        /// Source span.
        pos: Span,
    },
    /// An inline link: `[text](url)`.
    Link {
        /// Link text (inline children).
        text: Vec<Inline>,
        /// Destination URL.
        url: String,
        /// Optional link title.
        title: Option<String>,
        /// `true` if this is a GFM autolink (bare URL).
        autolink: bool,
        /// Source span.
        pos: Span,
    },
    /// A reference-style link: `[text][label]`, `[label][]`, or shortcut `[label]`.
    /// The `url` and `title` are resolved from the document's link reference
    /// definitions during parsing.
    LinkReference {
        /// Link text (inline children).
        text: Vec<Inline>,
        /// Normalised label used to look up the reference.
        label: String,
        /// Resolved destination URL.
        url: String,
        /// Optional resolved title.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Source span.
        pos: Span,
    },
}

/// Emphasis strength.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EmphasisLevel {
    /// `*italic*` / `_italic_`
    Italic,
    /// `**bold**` / `__bold__`
    Bold,
}
