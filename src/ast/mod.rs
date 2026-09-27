mod js;
mod options;
mod parsers;
pub mod tokens;

pub use options::AstOptions;
pub use parsers::Document;
pub use parsers::visitor;

use crate::frontmatter::parse_markdown_frontmatter;
use js::RegisteredJsVisitor;
pub use js::Visitor;
use napi::Env;
use parsers::visitor::visit_document_mut;
use serde_json::Value;
pub struct PeisarAst {
    /// The parsed AST document (private — use the `ast` getter
    /// [`get_ast`][Self::get_ast] or [`frontmatter`][Self::get_frontmatter]
    /// getters which auto-run visitors).
    ast: Document,
    /// Registered JS visitors (adapter wrappers).
    visitors: Vec<RegisteredJsVisitor>,
    /// Parsed YAML front-matter (if any).
    frontmatter: Option<Value>,
}

impl PeisarAst {
    /// If `options` is `None`, defaults are used (GFM + Kramdown enabled).
    pub fn new(raw_md: String, options: Option<AstOptions>) -> Self {
        let opts = options.unwrap_or_else(AstOptions::default);
        let f_n = opts.file_name.clone();

        // Parse front matter (reuse the same logic as PeisarAst).
        let (md_content, frontmatter) = match parse_markdown_frontmatter(&raw_md) {
            Ok(parsed) => parsed.into_parts(),
            Err(_) => (raw_md.clone(), None),
        };

        let ast = parsers::md_to_ast(&md_content, &opts, f_n);

        Self {
            ast,
            visitors: Vec::new(),
            frontmatter,
        }
    }

    /// Register a JS visitor (\"plugin\").
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
    pub fn add_visitor(&mut self, env: Env, visitor: Visitor) {
        self.visitors.push(visitor.register(env));
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
