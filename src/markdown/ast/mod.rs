mod control;
mod options;
mod parsers;
#[cfg(test)]
mod tests;
pub mod tokens;

use crate::frontmatter::frontmatter;
pub use control::{Parser, Visitor};
use control::{RegisteredParser, RegisteredVisitor};
#[cfg(feature = "npm")]
use napi::Env;
pub use options::AstOptions;
pub use parsers::Document;
pub use parsers::hooks::{AstParser, BlockParseContext, InlineParseContext, ParseHooks};
pub use parsers::inline::{LinkRefMap, parse_inline, parse_inline_with_refs};
use parsers::md_to_ast_with_hooks;
use parsers::visitor::visit_document_mut;
pub use parsers::visitor::{self, AstVisitor};
use serde_json::Value;
pub struct PeisarAst {
    /// The parsed AST document (private — use the `ast` getter
    /// [`get_ast`][Self::get_ast] or [`frontmatter`][Self::get_frontmatter]
    /// getters which auto-run visitors).
    ast: Document,
    /// Registered visitor adapters.
    visitors: Vec<RegisteredVisitor>,
    /// Parsed YAML front-matter (if any).
    frontmatter: Option<Value>,
    /// The raw Markdown source (kept for re-parsing when parser hooks are
    /// registered after construction).
    raw_md: String,
    /// Parsing options used for (re-)parsing.
    ast_opts: AstOptions,
    /// Registered parser hook adapters.
    parsers: Vec<RegisteredParser>,
}

impl PeisarAst {
    /// If `options` is `None`, defaults are used (GFM + Kramdown enabled).
    pub fn new(raw_md: String, options: Option<AstOptions>) -> Self {
        let opts = options.unwrap_or_else(AstOptions::default);
        let f_n = opts.file_name.clone();

        // Parse front matter (reuse the same logic as PeisarAst).
        let (md_content, frontmatter) = match frontmatter(raw_md.to_string()) {
            Ok(parsed) => parsed.into_parts(),
            Err(_) => (raw_md.clone(), None),
        };
        let ast = parsers::md_to_ast(&md_content, &opts, f_n);

        Self {
            ast,
            visitors: Vec::new(),
            frontmatter,
            raw_md,
            ast_opts: opts,
            parsers: Vec::new(),
        }
    }

    /// Register a visitor plugin.
    ///
    /// `visitor` is a plain JS object with two optional function
    /// properties:
    ///
    /// ```js
    /// ast.addVisitor({
    ///   visitBlock(block)   { return { recurse: true }; },
    ///   visitInline(inline) { return {}; },
    /// });
    /// ```
    ///
    /// Either callback may be `null` / omitted to skip that node kind.
    /// The JS function receives a `Block` or `Inline` and returns a
    /// `VisitControlJs` / `InlineVisitControlJs` (or `undefined`).
    #[cfg(feature = "npm")]
    pub fn add_visitor(&mut self, env: Env, visitor: Visitor) {
        self.visitors.push(visitor.register(env));
    }

    #[cfg(not(feature = "npm"))]
    pub fn add_visitor(&mut self, visitor: Visitor) {
        self.visitors.push(visitor.register());
    }

    /// Register a parser hook.
    ///
    /// `parser` is a plain JS object with two optional function properties:
    ///
    /// ```js
    /// ast.addParser({
    ///   parseBlock(ctx)  { /* returns { block, consumed } or undefined */ },
    ///   parseInline(ctx) { /* returns { inline, consumed } or undefined */ },
    /// });
    /// ```
    ///
    /// Because the document is parsed eagerly at construction, registering
    /// a parser hook re-parses the stored raw Markdown immediately (then
    /// re-runs any registered visitors on the fresh AST).
    #[cfg(feature = "npm")]
    pub fn add_parser(&mut self, env: Env, parser: Parser) {
        self.parsers.push(parser.register(env));
        self.reparse();
    }

    #[cfg(not(feature = "npm"))]
    pub fn add_parser(&mut self, parser: Parser) {
        self.parsers.push(parser.register());
        self.reparse();
    }

    /// Remove all registered parser hooks.
    pub fn clear_parsers(&mut self) {
        self.parsers.clear();
        self.reparse();
    }

    /// Re-parse the stored raw Markdown with the currently registered
    /// parser hooks, replacing the internal AST.  Front matter is
    /// re-extracted; registered visitors run afterwards on next access.
    pub fn reparse(&mut self) {
        let (md_content, frontmatter) = match frontmatter(self.raw_md.to_string()) {
            Ok(parsed) => parsed.into_parts(),
            Err(_) => (self.raw_md.clone(), None),
        };
        // Build the hook registry from the registered JS adapters.
        let mut hooks = ParseHooks::empty();
        for p in &self.parsers {
            hooks.push(p);
        }

        self.ast = md_to_ast_with_hooks(
            &md_content,
            &self.ast_opts.clone(),
            self.ast_opts.file_name.clone(),
            &hooks,
        );
        self.frontmatter = frontmatter;

        // Visitors registered before re-parse must re-apply to the fresh AST.
        for v in &mut self.visitors {
            visit_document_mut(&mut self.ast, v);
        }
    }

    /// Run all registered visitors in insertion order.
    ///
    /// This is called automatically by the [`ast`][Self::get_ast],
    /// [`ast_json`][Self::ast_json] and
    /// [`frontmatter`][Self::get_frontmatter] getters when there are
    /// registered visitors, but can also be called manually.
    pub fn visit_all(&mut self) {
        if self.visitors.is_empty() {
            return;
        }
        for v in &mut self.visitors {
            visit_document_mut(&mut self.ast, v);
        }
    }

    /// Remove all registered visitors.
    pub fn clear_visitors(&mut self) {
        self.visitors.clear();
    }

    /// Remove all registered visitors, running [`visit_all`](Self::visit_all)
    /// first so any pending mutations are applied.
    ///
    /// (Visitors are `ThreadsafeFunction` values which cannot be returned
    /// to JS, so this method returns nothing — use it purely for its
    /// side-effect of flushing + clearing the visitor list.)
    pub fn take_visitors(&mut self) {
        if !self.visitors.is_empty() {
            self.visit_all();
        }
        self.visitors.clear();
    }

    /// Borrow the parsed AST document as a JSON string.
    ///
    /// Runs [`visit_all`](Self::visit_all) first if there are registered
    /// visitors, so the returned AST always reflects visitor mutations.
    /// Prefer the `ast` getter (a real JS object) when you want object
    /// access on the JS side.
    pub fn ast_json(&mut self) -> String {
        if !self.visitors.is_empty() {
            self.visit_all();
        }
        serde_json::to_string(&self.ast).unwrap_or_default()
    }
    /// Consume and return the AST document, leaving a default [`Document`]
    /// in its place.
    ///
    /// Runs [`visit_all`](Self::visit_all) first if there are registered
    /// visitors, so the returned AST always reflects visitor mutations.
    /// After this call the internal AST is empty — use
    /// [`get_ast`][Self::get_ast] for a non-consuming clone.
    pub fn take_ast(&mut self) -> Document {
        if !self.visitors.is_empty() {
            self.visit_all();
        }
        std::mem::take(&mut self.ast)
    }

    /// Get a clone of the AST document as a real JS object.
    ///
    /// Runs [`visit_all`](Self::visit_all) first if there are registered
    /// visitors.  Unlike [`take_ast`](Self::take_ast) this does **not**
    /// consume the AST — subsequent calls still return the full tree.
    pub fn get_ast(&mut self) -> Document {
        if !self.visitors.is_empty() {
            self.visit_all();
        }
        self.ast.clone()
    }
    pub fn get_frontmatter(&mut self) -> Option<Value> {
        if !self.visitors.is_empty() {
            self.visit_all();
        }
        self.frontmatter.clone()
    }
}
