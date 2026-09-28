//! Node.js bindings for parsing Markdown into an AST and rendering it as HTML.
//!
//! The primary JavaScript API is [`Peisar`]. Construct it with Markdown and
//! optional [`PeisarOptions`], then read its `ast`, `html`, `frontmatter`, or
//! `astJson` properties.

mod ast;
mod config;
pub mod frontmatter;
mod html;
pub use config::PeisarOptions;
use napi::Env;
use napi_derive::napi;

use crate::html::{RenderOptions, render_document_html};
pub use ast::{
    AstOptions, AstParser, AstVisitor, BlockParseContext, Document, InlineParseContext,
    ParseHooks, PeisarAst, Parser, Visitor,
    tokens::{Attributes, span, token},
    visitor,
};
use config::get_options;
use serde_json::Value;

/// Parses one Markdown document and exposes its AST, HTML, and front matter
/// to JavaScript.
///
/// Construct this class with the Markdown source and an optional
/// [`PeisarOptions`] object. Properties are computed from the source document;
/// accessing `ast`, `html`, or `astJson` also applies registered visitors.
#[napi]
pub struct Peisar {
    render_opts: RenderOptions,
    peisar_ast: PeisarAst,
}

#[napi]
impl Peisar {
    /// Creates a parser for `rawMd`.
    ///
    /// When `options` is omitted, GFM and Kramdown parsing are enabled and
    /// `html` returns a complete HTML document.
    #[napi(constructor)]
    pub fn new(raw_md: String, options: Option<PeisarOptions>) -> Self {
        let opts = get_options(options);
        let ast_opts = opts.ast_opts;
        let render_opts = opts.render_opts;
        let peisar_ast = PeisarAst::new(raw_md, Some(ast_opts));
        Self {
            render_opts,
            peisar_ast,
        }
    }

    /// Gets the parsed Markdown document as a JavaScript AST object.
    ///
    /// Registered visitors run before the AST is returned. The returned value
    /// is a clone, so reading this property does not consume the parser state.
    #[napi(getter)]
    pub fn ast(&mut self) -> Document {
        self.peisar_ast.get_ast()
    }

    /// Registers a JavaScript AST visitor.
    ///
    /// The visitor can provide `visitBlock` and/or `visitInline` callbacks.
    /// Each callback receives a one-item node tuple and may return a control object that
    /// changes the node or determines whether its children are visited.
    #[napi]
    pub fn use_visitor(&mut self, env: Env, visitor: Visitor) {
        self.peisar_ast.add_visitor(env, visitor);
    }

    /// Registers a JavaScript custom parser hook.
    ///
    /// The parser can provide `parseBlock` and/or `parseInline` callbacks.
    /// `parseBlock` receives `{ line, lineIndex, lines }` and returns
    /// `{ block, consumed }`; `parseInline` receives `{ rest, index }` and
    /// returns `{ inline, consumed }`.  Return `undefined` (or omit the
    /// node) to decline a position.
    ///
    /// Hooks run before the built-in parsers, in registration order.  Since
    /// the document is parsed at construction, registering a parser hook
    /// re-parses the original Markdown so hook syntax is recognized.
    #[napi]
    pub fn use_parser(&mut self, env: Env, parser: Parser) {
        self.peisar_ast.add_parser(env, parser);
    }

    /// Renders the parsed Markdown as HTML using the configured render options.
    ///
    /// Registered visitors run before rendering.
    #[napi(getter)]
    pub fn html(&mut self) -> String {
        let doc = self.ast();
        let html = render_document_html(&doc, Some(self.render_opts.clone()));
        html
    }

    /// Gets the deserialized YAML front matter, or `null` when none exists.
    #[napi(getter)]
    pub fn frontmatter(&mut self) -> Option<Value> {
        self.peisar_ast.get_frontmatter()
    }

    /// Gets the parsed Markdown AST serialized as JSON.
    ///
    /// Registered visitors run before serialization.
    #[napi(getter)]
    pub fn ast_json(&mut self) -> String {
        self.peisar_ast.ast_json()
    }
}
