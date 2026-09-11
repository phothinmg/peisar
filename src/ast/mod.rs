//! Markdown AST node definitions.
//!
//! The AST is divided into sub-modules for clarity:
//! - [`nodes`] — all block-level and inline-level node types.
//! - [`span`] — source position tracking (`Position`, `Span`).
//!
//! Each node variant corresponds to a Markdown construct from CommonMark,
//! GitHub Flavored Markdown (GFM), or a Kramdown-style extension.

pub mod nodes;
pub mod span;
pub use nodes::{
    Block, Document, EmphasisLevel, Inline, KramdownAttributes, ListItem, Table, TableCell,
    TableCellAlignment, TableRow, TaskState,
};
pub use span::{Position, Span};
