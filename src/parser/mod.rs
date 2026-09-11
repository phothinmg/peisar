//! Markdown parser — produces the AST defined in [`crate::ast`].
//!
//! The parser is split into sub-modules for maintainability:
//!
//! | Module      | Responsibility                                         |
//! |-------------|--------------------------------------------------------|
//! | [`block`]   | Block-level parsing (headings, code, lists, tables …)  |
//! | [`inline`]  | Inline-level parsing (emphasis, links, code, strikethrough …) |
//! | [`kramdown`]| Kramdown `{:#id .class key="val"}` attribute parsing   |
//! | [`gfm`]     | GFM-specific helpers (tables, task lists, autolinks)    |
//!
//! The top-level [`parse`] function is the main entry point.

pub mod block;
pub mod gfm;
pub mod inline;
pub mod kramdown;

use crate::ast::Document;
use crate::config::ParseOptions;

/// Parse a Markdown string into a [`Document`] using default options.
pub fn parse(input: &str) -> Document {
    parse_with(input, &ParseOptions::default())
}

/// Parse a Markdown string into a [`Document`] with the given options.
///
/// Options control which extensions (GFM, Kramdown) are active.
pub fn parse_with(input: &str, opts: &ParseOptions) -> Document {
    let line_starts = block::compute_line_starts(input);
    let lines: Vec<&str> = input.lines().collect();
    let mut p = block::ParserState::new(input, &lines, &line_starts, opts);
    let start = p.position_at(0);

    let mut blocks = Vec::new();

    while !p.is_done() {
        if p.skip_blank_lines() {
            continue;
        }
        if let Some(block) = p.parse_block() {
            blocks.push(block);
        } else {
            p.advance(); // safety net — never loop forever
        }
    }

    let end = p.position_at(p.pos.min(p.lines.len()));

    Document {
        node_type: "root",
        children: blocks,
        position: crate::ast::Span::new(start, end),
    }
}
