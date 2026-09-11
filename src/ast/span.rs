//! Source position tracking for AST nodes.
//!
//! Every node in the AST carries a [`Span`] so that tools (linters,
//! language servers, round-trip renderers) can map back to the original
//! source text.
//!
//! Positions are **0-based**:
//! - `line` is the line index (0 = first line),
//! - `column` is the character column within that line,
//! - `offset` is the byte offset from the start of the input.

use serde::Serialize;

/// A zero-based point in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Position {
    /// Line number, 0-based.
    pub line: usize,
    /// Column number, 0-based (in characters).
    pub column: usize,
    /// Byte offset from the start of the input.
    pub offset: usize,
}

/// A half-open span `[start, end)` covering a node's source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }
}
