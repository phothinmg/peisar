//! Markdown parsers — block-level, inline-level, tables, and plugin hooks.
//!
//! This module is the parsing core of `peisar_ast`.  It exposes the
//! [`Document`] root type and the [`md_to_ast`] entry-point function.
//!
//! ## Sub-modules
//!
//! - [`block`] — block-level parser (headings, code, lists, block quotes,
//!   tables, link reference definitions, HTML comments).
//! - [`inline`] — inline-level parser (emphasis, code, links, images,
//!   reference links, autolink, strikethrough).
//! - `table` — GFM table parsing helpers.
//! - `attrs` — Kramdown block attribute parser.
//! - [`plugin`] — parser hooks, visitor callbacks, and AST traversal.

pub mod attrs;
pub mod block;
pub mod inline;
/// Parser hooks, visitor callbacks, and AST traversal utilities.
pub mod plugin;
pub mod table;

use crate::peisar_core::options::AstOptions;
use crate::peisar_core::parsers::plugin::ParserHooks;
use crate::peisar_core::tokens::{span::Span, token::Block};
use inline::LinkRefMap;
#[cfg(feature = "npm")]
use napi_derive::napi;
use serde::Serialize;

// ---------------------------------------------------------------------------
// PeisarDocument
// ---------------------------------------------------------------------------

/// A Markdown document — the root of the AST.
///
/// Contains the top-level block children and all link reference definitions
/// collected from the source text.
///

#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PeisarDocument {
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
    pub link_references: Vec<crate::peisar_core::tokens::token::LinkReferenceDefinition>,
}

impl Default for PeisarDocument {
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

impl PeisarDocument {
    /// Parse a Markdown string into a [`PeisarDocument`] with the given options.
    ///
    /// This is a convenience wrapper around the module's `md_to_ast` parser.
    ///
    pub fn parse(input: &str, opts: &AstOptions, file_name: Option<String>) -> Self {
        md_to_ast(input, opts, file_name)
    }
}
/// Parse a Markdown string into a [`PeisarDocument`] with the given options.
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
/// - `file_name` — optional file name to attach to the resulting [`PeisarDocument`].
///
pub fn md_to_ast(input: &str, opts: &AstOptions, file_name: Option<String>) -> PeisarDocument {
    md_to_ast_with_hooks(input, opts, file_name, &ParserHooks::empty())
}
/// Like `md_to_ast` but with custom parser hooks.
///
/// Hooks run before the built-in block and inline matchers; the first hook
/// to claim a position wins.
pub fn md_to_ast_with_hooks(
    input: &str,
    opts: &AstOptions,
    file_name: Option<String>,
    hooks: &ParserHooks,
) -> PeisarDocument {
    let line_starts = block::compute_line_starts(input);
    let lines: Vec<&str> = input.lines().collect();

    // Pre-pass: collect all link reference definitions into a map so that
    // reference-style links can be resolved during inline parsing.
    let mut ref_map: LinkRefMap = LinkRefMap::new();
    let mut ref_defs: Vec<crate::peisar_core::tokens::token::LinkReferenceDefinition> = Vec::new();
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

    PeisarDocument {
        node_type: "root".to_string(),
        pos: Span::new(start, end),
        file_name,
        children: blocks,
        link_references: ref_defs,
    }
}
