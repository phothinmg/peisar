mod control;
mod options;
mod parsers;
#[cfg(test)]
mod tests;
pub mod tokens;

use crate::frontmatter::frontmatter;
pub use control::Plugin;
pub(crate) use control::RegisteredPlugin;
#[allow(deprecated)]
pub use control::{Parser, Visitor};
#[cfg(feature = "npm")]
use napi::Env;
pub use options::AstOptions;
pub use parsers::Document;
pub use parsers::inline::{LinkRefMap, parse_inline, parse_inline_with_refs};
pub use parsers::md_to_ast_with_hooks;
pub use parsers::plugin::{ParserHooks, PluginFactory, visit_document_mut};
// Deprecated API since v1.3.0 — kept for backwards compatibility.
/// Deprecated pre-1.3.0 traversal — drove an [`AstVisitor`] over a
/// document.  New code implements [`PluginFactory`] and calls
/// [`visit_document_mut`] instead.
#[allow(deprecated)]
#[deprecated(
    since = "1.3.0",
    note = "Use `visit_document_mut` with a `PluginFactory` instead"
)]
pub use parsers::plugin::visit_document_mut_dep;
#[allow(deprecated)]
pub use parsers::plugin::{
    AstParser, AstParserAdapter, AstVisitor, AstVisitorAdapter, BlockParseContext,
    InlineParseContext, ParseHooks,
};
use serde_json::Value;
pub struct PeisarAst {
    /// The parsed AST document (private — use the `ast` getter
    /// [`get_ast`][Self::get_ast] or [`frontmatter`][Self::get_frontmatter]
    /// getters which auto-run visitors).
    ast: Document,
    /// Parsed YAML front-matter (if any).
    frontmatter: Option<Value>,
    /// The raw Markdown source (kept for re-parsing when parser hooks are
    /// registered after construction).
    raw_md: String,
    /// Parsing options used for (re-)parsing.
    ast_opts: AstOptions,
    /// Registered plugin adapters (unified API; deprecated `Visitor` /
    /// `Parser` registrations are converted into these at registration
    /// time, so a single code path drives everything).
    plugins: Vec<RegisteredPlugin>,
    /// Whether the registered plugins have already been applied to the
    /// current AST — getters apply them exactly once instead of stacking
    /// visitor mutations on every access.
    plugins_applied: bool,
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
            frontmatter,
            raw_md,
            ast_opts: opts,
            plugins: Vec::new(),
            plugins_applied: true,
        }
    }
    // New Plugin API

    #[cfg(feature = "npm")]
    pub fn add_plugin(&mut self, env: Env, plugin: Plugin) {
        self.plugins.push(plugin.register(env));
        self.plugins_applied = false;
    }

    #[cfg(not(feature = "npm"))]
    pub fn add_plugin(&mut self, plugin: Plugin) {
        self.plugins.push(plugin.register());
        self.plugins_applied = false;
    }

    /// Re-parse the stored raw Markdown with the currently registered
    /// plugins' `parseBlock` / `parseInline` hooks, replacing the internal
    /// AST.  Front matter is re-extracted; each plugin's `visitBlock` /
    /// `visitInline` callbacks run on the fresh AST immediately after.
    fn parse_plugin(&mut self) {
        let (md_content, frontmatter) = match frontmatter(self.raw_md.to_string()) {
            Ok(parsed) => parsed.into_parts(),
            Err(_) => (self.raw_md.clone(), None),
        };
        // Build the hook registry from the registered JS adapters.
        let mut hooks = ParserHooks::empty();
        for p in &self.plugins {
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
        for v in &mut self.plugins {
            visit_document_mut(&mut self.ast, v);
        }
    }

    /// Apply the registered plugins exactly once per registration set:
    /// re-parses when any plugin carries parse hooks, then runs visit
    /// callbacks.  Idempotent — repeated getter calls do not stack
    /// visitor mutations.
    fn ensure_plugins_applied(&mut self) {
        if self.plugins_applied || self.plugins.is_empty() {
            return;
        }
        let needs_reparse = self.plugins.iter().any(|p| {
            let p = p as &dyn PluginFactory;
            p.has_parse_hooks()
        });
        if needs_reparse {
            self.parse_plugin();
        } else {
            for v in &mut self.plugins {
                visit_document_mut(&mut self.ast, v);
            }
        }
        self.plugins_applied = true;
    }

    // Deprecated API since v1.3.0

    /// Register a visitor plugin (deprecated form of
    /// [`add_plugin`](Self::add_plugin)).
    #[cfg(feature = "npm")]
    #[allow(deprecated)]
    #[deprecated(since = "1.3.0", note = "Use `add_plugin` instead")]
    pub fn add_visitor(&mut self, env: Env, visitor: Visitor) {
        self.plugins.push(visitor.into_plugin(env));
        self.plugins_applied = false;
    }

    #[cfg(not(feature = "npm"))]
    #[allow(deprecated)]
    #[deprecated(since = "1.3.0", note = "Use `add_plugin` instead")]
    pub fn add_visitor(&mut self, visitor: Visitor) {
        self.plugins.push(visitor.into_plugin());
        self.plugins_applied = false;
    }

    /// Register a parser hook (deprecated form of
    /// [`add_plugin`](Self::add_plugin)).
    ///
    /// Because the document is parsed eagerly at construction, registering
    /// a parser hook re-parses the stored raw Markdown immediately (then
    /// re-runs any registered visitors on the fresh AST).
    #[cfg(feature = "npm")]
    #[allow(deprecated)]
    #[deprecated(since = "1.3.0", note = "Use `add_plugin` instead")]
    pub fn add_parser(&mut self, env: Env, parser: Parser) {
        self.plugins.push(parser.into_plugin(env));
        self.plugins_applied = false;
    }

    #[cfg(not(feature = "npm"))]
    #[allow(deprecated)]
    #[deprecated(since = "1.3.0", note = "Use `add_plugin` instead")]
    pub fn add_parser(&mut self, parser: Parser) {
        self.plugins.push(parser.into_plugin());
        self.plugins_applied = false;
    }

    /// Re-parse the stored raw Markdown with the currently registered
    /// parser hooks, replacing the internal AST.  Front matter is
    /// re-extracted; registered visitors run afterwards on next access.
    #[deprecated(
        since = "1.3.0",
        note = "Plugins re-parse automatically — call `add_plugin` instead"
    )]
    pub fn reparse(&mut self) {
        self.parse_plugin();
        self.plugins_applied = true;
    }

    /// Remove all registered plugins (deprecated name kept for
    /// compatibility — visitors and parsers are unified now).
    #[deprecated(since = "1.3.0", note = "Use `clear_plugins` instead")]
    pub fn clear_parsers(&mut self) {
        self.clear_plugins();
    }
    /// Run all registered plugins' visitor callbacks in insertion order.
    ///
    /// This is called automatically by the [`ast`][Self::get_ast],
    /// [`ast_json`][Self::ast_json] and
    /// [`frontmatter`][Self::get_frontmatter] getters when there are
    /// registered plugins, but can also be called manually.
    #[deprecated(since = "1.3.0", note = "Plugins auto-apply — use getters directly")]
    pub fn visit_all(&mut self) {
        self.ensure_plugins_applied();
    }

    /// Remove all registered plugins.
    pub fn clear_plugins(&mut self) {
        self.plugins.clear();
        self.plugins_applied = true;
    }

    /// Remove all registered plugins, running their visitor callbacks
    /// first so any pending mutations are applied.
    ///
    /// (Callbacks are `FunctionRef` values which cannot be returned to JS,
    /// so this method returns nothing — use it purely for its side-effect
    /// of flushing + clearing the plugin list.)
    #[deprecated(since = "1.3.0", note = "Use `clear_plugins` instead")]
    pub fn take_visitors(&mut self) {
        self.ensure_plugins_applied();
        self.clear_plugins();
    }

    /// Remove all registered plugins (deprecated alias of
    /// [`clear_plugins`](Self::clear_plugins)).
    #[deprecated(since = "1.3.0", note = "Use `clear_plugins` instead")]
    pub fn clear_visitors(&mut self) {
        self.clear_plugins();
    }

    // Getters

    /// Borrow the parsed AST document as a JSON string.
    ///
    /// Registered plugins are applied first (exactly once) so the JSON
    /// always reflects plugin mutations.
    pub fn ast_json(&mut self) -> String {
        self.ensure_plugins_applied();
        serde_json::to_string(&self.ast).unwrap_or_default()
    }
    /// Consume and return the AST document, leaving a default [`Document`]
    /// in its place.
    ///
    /// Applies registered plugins first, so the returned AST always
    /// reflects plugin mutations.  After this call the internal AST is
    /// empty — use [`get_ast`][Self::get_ast] for a non-consuming clone.
    pub fn take_ast(&mut self) -> Document {
        self.ensure_plugins_applied();
        std::mem::take(&mut self.ast)
    }

    /// Get a clone of the AST document as a real JS object.
    ///
    /// Applies registered plugins first.  Unlike [`take_ast`](Self::take_ast)
    /// this does **not** consume the AST — subsequent calls still return
    /// the full tree.
    pub fn get_ast(&mut self) -> Document {
        self.ensure_plugins_applied();
        self.ast.clone()
    }
    pub fn get_frontmatter(&mut self) -> Option<Value> {
        self.ensure_plugins_applied();
        self.frontmatter.clone()
    }
}
