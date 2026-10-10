//! Node.js bindings for parsing Markdown into an AST and rendering it as HTML.
//!
//! The primary JavaScript API is [`Peisar`]. Construct it with Markdown and
//! optional [`PeisarOptions`], then read its `ast`, `html`, `frontmatter`, or
//! `astJson` properties.

#[allow(deprecated)]
use crate::markdown::{
    ast::{Document, Parser, PeisarAst, Plugin, Visitor},
    config::{PeisarOptions, get_options},
    html::{RenderOptions, render_document_html},
};
#[cfg(feature = "npm")]
use napi::Env;
#[cfg(feature = "npm")]
use napi_derive::napi;
use serde_json::Value;

/// Parses one Markdown document and exposes its AST, HTML, and front matter
/// to JavaScript.
///
/// Construct this class with the Markdown source and an optional
/// [`PeisarOptions`] object. Properties are computed from the source document;
/// accessing `ast`, `html`, or `astJson` also applies registered visitors.
#[cfg_attr(feature = "npm", napi)]
pub struct Peisar {
    render_opts: RenderOptions,
    peisar_ast: PeisarAst,
}
#[cfg_attr(feature = "npm", napi)]
impl Peisar {
    /// Creates a parser for `rawMd`.
    ///
    /// When `options` is omitted, GFM and Kramdown parsing are enabled and
    /// `html` returns a complete HTML document.
    #[cfg_attr(feature = "npm", napi(constructor))]
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
    #[cfg_attr(feature = "npm", napi(getter))]
    pub fn ast(&mut self) -> Document {
        self.peisar_ast.get_ast()
    }

    /// Registers a unified JavaScript plugin.
    ///
    /// The plugin can provide `visitBlock` / `visitInline` visitor callbacks
    /// and `parseBlock` / `parseInline` parser hooks, all in one object.
    #[cfg(feature = "npm")]
    #[napi]
    pub fn use_plugin(&mut self, env: Env, plugin: Plugin) {
        self.peisar_ast.add_plugin(env, plugin);
    }
    #[cfg(not(feature = "npm"))]
    pub fn use_plugin(&mut self, plugin: Plugin) {
        self.peisar_ast.add_plugin(plugin);
    }

    /// Registers a JavaScript AST visitor (deprecated since 1.3.0 — use
    /// [`usePlugin`](Self::use_plugin) instead).
    ///
    /// The visitor can provide `visitBlock` and/or `visitInline` callbacks.
    /// Each callback receives a one-item node tuple and may return a control object that
    /// changes the node or determines whether its children are visited.
    #[cfg(feature = "npm")]
    #[napi]
    #[allow(deprecated)]
    pub fn use_visitor(&mut self, env: Env, visitor: Visitor) {
        self.peisar_ast.add_visitor(env, visitor);
    }
    #[cfg(not(feature = "npm"))]
    #[allow(deprecated)]
    pub fn use_visitor(&mut self, visitor: Visitor) {
        self.peisar_ast.add_visitor(visitor);
    }

    /// Registers a JavaScript custom parser hook (deprecated since 1.3.0 —
    /// use [`usePlugin`](Self::use_plugin) instead).
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
    #[cfg(feature = "npm")]
    #[napi]
    #[allow(deprecated)]
    pub fn use_parser(&mut self, env: Env, parser: Parser) {
        self.peisar_ast.add_parser(env, parser);
    }
    #[cfg(not(feature = "npm"))]
    #[allow(deprecated)]
    pub fn use_parser(&mut self, parser: Parser) {
        self.peisar_ast.add_parser(parser);
    }
    /// Renders the parsed Markdown as HTML using the configured render options.
    ///
    /// Registered visitors run before rendering.
    #[cfg_attr(feature = "npm", napi(getter))]
    pub fn html(&mut self) -> String {
        let doc = self.ast();
        let html = render_document_html(&doc, Some(self.render_opts.clone()));
        html
    }

    /// Gets the deserialized YAML front matter, or `null` when none exists.
    #[cfg_attr(feature = "npm", napi(getter))]
    pub fn frontmatter(&mut self) -> Option<Value> {
        self.peisar_ast.get_frontmatter()
    }

    /// Gets the parsed Markdown AST serialized as JSON.
    ///
    /// Registered visitors run before serialization.
    #[cfg_attr(feature = "npm", napi(getter))]
    pub fn ast_json(&mut self) -> String {
        self.peisar_ast.ast_json()
    }
}
