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
#[cfg(feature = "npm")]
use napi_derive::napi;
use serde::Serialize;

/// A zero-based point in the source text.
#[cfg_attr(feature = "npm", napi(object))]
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
/// `start` is inclusive and `end` is exclusive; both positions point into the
/// original source string.
#[cfg_attr(feature = "npm", napi(object))]
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
    ///
    /// Values are stored as `u32`; panics on overflow (only possible for
    /// inputs larger than 4 GiB).
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line: line.try_into().unwrap(),
            column: column.try_into().unwrap(),
            offset: offset.try_into().unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Position, Span};

    #[test]
    fn defaults_point_to_the_document_start() {
        let pos = Position::default();
        assert_eq!((pos.line, pos.column, pos.offset), (0, 0, 0));

        let span = Span::default();
        assert_eq!(span.start, pos);
        assert_eq!(span.end, pos);
    }

    #[test]
    fn new_stores_all_three_coordinates() {
        let pos = Position::new(3, 7, 42);
        assert_eq!(pos.line, 3);
        assert_eq!(pos.column, 7);
        assert_eq!(pos.offset, 42);
    }

    #[test]
    fn span_new_builds_half_open_range() {
        let start = Position::new(0, 4, 4);
        let end = Position::new(0, 9, 9);
        let span = Span::new(start, end);
        assert_eq!(span.start, start);
        assert_eq!(span.end, end);
    }

    #[test]
    fn spans_compare_by_value() {
        let a = Span::new(Position::new(1, 0, 5), Position::new(1, 3, 8));
        let b = Span::new(Position::new(1, 0, 5), Position::new(1, 3, 8));
        let c = Span::new(Position::new(1, 0, 5), Position::new(1, 4, 9));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
