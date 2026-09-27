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

use napi_derive::napi;
use serde::Serialize;

/// A zero-based point in the source text.
#[napi(object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Position {
    /// Line number, 0-based.
    pub line: u32,
    /// Column number, 0-based (in characters).
    pub column: u32,
    /// Byte offset from the start of the input.
    pub offset: u32,
}

/// A half-open span `[start, end)` covering a node's source text.
///
/// Both `start` and `end` are inclusive [`Position`]s that point into the
/// original source string.
#[napi(object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Span {
    /// The start position (inclusive).
    pub start: Position,
    /// The end position (exclusive).
    pub end: Position,
}

impl Span {
    /// Create a new span from `start` to `end`.
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

impl Position {
    /// Create a new position at the given `line`, `column`, and byte `offset`.
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line: line.try_into().unwrap(),
            column: column.try_into().unwrap(),
            offset: offset.try_into().unwrap(),
        }
    }
}
