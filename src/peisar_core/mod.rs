// #![deny(missing_docs)]

//! Markdown parsing into a structured AST, with optional parser and visitor
//! plugins.
//!
//! [`PeisarAst`] parses Markdown and provides access to its AST and raw
//! front matter. Use [`PeisarPlugin`] to customize parsing or mutate AST nodes.
//!
//! # Example
//!
//! ```
//! use peisar::ast::PeisarAst;
//!
//! let mut parsed = PeisarAst::new("# Hello".to_string(), None);
//! let PeisarDocument = parsed.get_ast();
//! assert_eq!(PeisarDocument.children.len(), 1);
//! ```

pub mod control;
pub mod frontmatter;
pub mod options;
pub mod parsers;
#[cfg(test)]
mod tests;
pub mod tokens;

pub use control::{
    BlockParseResult, InlineParseResult, InlineVisitorControl, PeisarPlugin, RegisteredPlugin,
    VisitorControl,
};
use frontmatter::{Frontmatter, extract_frontmatter};
#[cfg(feature = "npm")]
use napi::Env;
#[cfg(feature = "npm")]
use napi_derive::napi;
use options::AstOptions;
use parsers::{PeisarDocument, md_to_ast_with_hooks, plugin};

/// Markdown source parsed into an AST, with optional raw front matter and
/// registered plugins.
#[cfg_attr(feature = "npm", napi)]
pub struct PeisarAst {
    /// The parsed AST PeisarDocument (private — use the `ast` getter
    /// [`get_ast`][Self::get_ast] or [`frontmatter`][Self::get_frontmatter]
    /// getters which auto-run visitors).
    ast: PeisarDocument,
    /// Extracted frontmatter (if present).
    frontmatter: Frontmatter,
    /// The raw Markdown source (kept for re-parsing when parser hooks are
    /// registered after construction).
    raw_md: String,
    /// Parsing options used for (re-)parsing.
    ast_opts: AstOptions,
    /// Registered plugins.
    plugins: Vec<RegisteredPlugin>,
    /// Whether the registered plugins have already been applied to the
    /// current AST — getters apply them exactly once instead of stacking
    /// visitor mutations on every access.
    plugins_applied: bool,
}
#[cfg_attr(feature = "npm", napi)]
impl PeisarAst {
    /// If `options` is `None`, defaults are used (GFM + Kramdown enabled).
    #[cfg_attr(feature = "npm", napi(constructor))]
    pub fn new(raw_md: String, options: Option<AstOptions>) -> Self {
        let opts = options.unwrap_or_else(AstOptions::default);
        let f_n = opts.file_name.clone();

        // Extract front matter before parsing the Markdown body.
        let fmt = extract_frontmatter(raw_md.to_string());
        let ast = parsers::md_to_ast(&fmt.body, &opts, f_n);

        Self {
            ast,
            frontmatter: fmt.frontmatter,
            raw_md,
            ast_opts: opts,
            plugins: Vec::new(),
            plugins_applied: true,
        }
    }
    /// Register a plugin whose parser hooks and visitor callbacks run when
    /// the AST is next accessed.
    #[cfg(feature = "npm")]
    pub fn add_plugin(&mut self, env: Env, plugin: PeisarPlugin) {
        self.plugins.push(plugin.register(env));
        self.plugins_applied = false;
    }

    #[cfg(not(feature = "npm"))]
    /// Register a plugin whose parser hooks and visitor callbacks run when
    /// the AST is next accessed.
    pub fn add_plugin(&mut self, plugin: PeisarPlugin) {
        self.plugins.push(plugin.register());
        self.plugins_applied = false;
    }

    /// Re-parse the stored raw Markdown with the currently registered
    /// plugins' `parseBlock` / `parseInline` hooks, replacing the internal
    /// AST.  Front matter is re-extracted; each plugin's `visitBlock` /
    /// `visitInline` callbacks run on the fresh AST immediately after.
    fn parse_plugin(&mut self) {
        let fmt = extract_frontmatter(self.raw_md.to_string());
        // Build the hook registry from the registered JS adapters.
        let mut hooks = plugin::ParserHooks::empty();
        for p in &self.plugins {
            hooks.push(p);
        }

        self.ast = md_to_ast_with_hooks(
            &fmt.body,
            &self.ast_opts.clone(),
            self.ast_opts.file_name.clone(),
            &hooks,
        );
        self.frontmatter = fmt.frontmatter;

        // Visitors registered before re-parse must re-apply to the fresh AST.
        for v in &mut self.plugins {
            plugin::visit_document_mut(&mut self.ast, v);
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
            let p = p as &dyn plugin::PluginFactory;
            p.has_parse_hooks()
        });
        if needs_reparse {
            self.parse_plugin();
        } else {
            for v in &mut self.plugins {
                plugin::visit_document_mut(&mut self.ast, v);
            }
        }
        self.plugins_applied = true;
    }

    /// Remove all registered plugins.
    #[cfg_attr(feature = "npm", napi)]
    pub fn clear_plugins(&mut self) {
        self.plugins.clear();
        self.plugins_applied = true;
    }
    /// Consume and return the AST PeisarDocument, leaving a default [`PeisarDocument`]
    /// in its place.
    ///
    /// Applies registered plugins first, so the returned AST always
    /// reflects plugin mutations.  After this call the internal AST is
    /// empty — use [`get_ast`][Self::get_ast] for a non-consuming clone.
    pub fn take_ast(&mut self) -> PeisarDocument {
        self.ensure_plugins_applied();
        std::mem::take(&mut self.ast)
    }

    /// Get a clone of the AST PeisarDocument as a real JS object.
    ///
    /// Applies registered plugins first.  Unlike [`take_ast`](Self::take_ast)
    /// this does **not** consume the AST — subsequent calls still return
    /// the full tree.
    #[cfg_attr(feature = "npm", napi)]
    pub fn get_ast(&mut self) -> PeisarDocument {
        self.ensure_plugins_applied();
        self.ast.clone()
    }
    /// Return a clone of the extracted frontmatter.
    ///
    /// Applies registered plugins first, matching [`get_ast`](Self::get_ast).
    #[cfg_attr(feature = "npm", napi)]
    pub fn get_frontmatter(&mut self) -> Frontmatter {
        self.ensure_plugins_applied();
        self.frontmatter.clone()
    }
}
