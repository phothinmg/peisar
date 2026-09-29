//! Markdown parsers — block-level, inline-level, tables, and visitors.
//!
//! This module is the parsing core of `peisar_ast`.  It exposes the
//! [`Document`] root type and the [`md_to_ast`] entry-point function.
//!
//! ## Sub-modules
//!
//! - [`block`] — block-level parser (headings, code, lists, block quotes,
//!   tables, link reference definitions, HTML comments).
//! - [`inline`] — inline-level parser (emphasis, code, links, images,
//!   reference links, autolinks, strikethrough).
//! - `table` — GFM table parsing helpers.
//! - `atters` — Kramdown block attribute parser.
//! - [`visitor`] — the [`AstVisitor`](visitor::AstVisitor) trait and
//!   [`visit_document_mut`](visitor::visit_document_mut) traversal engine.

mod atters;
pub mod block;
pub mod hooks;
pub mod inline;
mod table;
pub mod visitor;

use crate::markdown::ast::options::AstOptions;
use crate::markdown::ast::parsers::hooks::ParseHooks;
use crate::markdown::ast::tokens::{span::Span, token::Block};
use inline::LinkRefMap;
use napi_derive::napi;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

/// A Markdown document — the root of the AST.
///
/// Contains the top-level block children and all link reference definitions
/// collected from the source text.
///

#[napi(object)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Document {
    /// Always `"root"`.
    #[serde(rename = "type")]
    pub node_type: String,
    /// Optional source file name.
    pub file_name: Option<String>,
    /// Span of the whole document in the source text.
    pub pos: Span,
    /// Top-level block children.
    pub children: Vec<Block>,
    /// All link reference definitions collected from the document.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub link_references: Vec<crate::markdown::ast::tokens::token::LinkReferenceDefinition>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            node_type: "root".to_string(),
            file_name: None,
            pos: Span::default(),
            children: Vec::new(),
            link_references: Vec::new(),
        }
    }
}

impl Document {
    /// Parse a Markdown string into a [`Document`] with the given options.
    ///
    /// This is a convenience wrapper around the module's `md_to_ast` parser.
    ///
    pub fn parse(input: &str, opts: &AstOptions, file_name: Option<String>) -> Self {
        md_to_ast(input, opts, file_name)
    }
}
/// Parse a Markdown string into a [`Document`] with the given options.
///
/// This is the primary entry point for the parser.  It performs a pre-pass
/// to collect all link reference definitions (so that reference-style links
/// can be resolved during inline parsing) and then parses the document into
/// block-level children.
///
/// # Arguments
///
/// - `input` — the raw Markdown source text (without front-matter).
/// - `opts` — parser options (GFM, Kramdown, etc.).
/// - `file_name` — optional file name to attach to the resulting [`Document`].
///
pub fn md_to_ast(input: &str, opts: &AstOptions, file_name: Option<String>) -> Document {
    md_to_ast_with_hooks(input, opts, file_name, &ParseHooks::empty())
}

/// Like [`md_to_ast`] but with custom parser hooks (see
/// [`hooks`](self::hooks)).
///
/// Hooks run before the built-in block and inline matchers; the first hook
/// to claim a position wins.
pub fn md_to_ast_with_hooks(
    input: &str,
    opts: &AstOptions,
    file_name: Option<String>,
    hooks: &ParseHooks,
) -> Document {
    let line_starts = block::compute_line_starts(input);
    let lines: Vec<&str> = input.lines().collect();

    // Pre-pass: collect all link reference definitions into a map so that
    // reference-style links can be resolved during inline parsing.
    let mut ref_map: LinkRefMap = LinkRefMap::new();
    let mut ref_defs: Vec<crate::markdown::ast::tokens::token::LinkReferenceDefinition> =
        Vec::new();
    for (line_idx, line) in lines.iter().enumerate() {
        if block::is_link_ref_def(line) {
            if let Some(def) = block::parse_link_ref_def_line(line, line_idx, &line_starts, input) {
                ref_map.insert(def.label.clone(), (def.url.clone(), def.title.clone()));
                ref_defs.push(def);
            }
        }
    }

    let mut p = block::ParserState::new(
        input,
        &lines,
        &line_starts,
        opts,
        file_name.clone(),
        &ref_map,
        hooks,
    );
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
        node_type: "root".to_string(),
        pos: Span::new(start, end),
        file_name,
        children: blocks,
        link_references: ref_defs,
    }
}
