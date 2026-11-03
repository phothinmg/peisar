use super::config::{PeisarOptions, RenderOptions, get_options};
use super::html::render_document_html;
#[cfg(feature = "npm")]
use napi::Env;
#[cfg(feature = "npm")]
use napi_derive::napi;
use peisar_ast::{Document, PeisarAst, Plugin};
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
    /// Visitor callbacks receive their node as a one-item tuple: for example,
    /// `visitBlock([block]) { return { recurse: true }; }`.
    #[cfg(feature = "npm")]
    #[napi(js_name = "use")]
    pub fn use_plugin(&mut self, env: Env, plugin: Plugin) {
        self.peisar_ast.add_plugin(env, plugin);
    }
    /// Registers a plugin that can customize parsing and visit AST nodes.
    ///
    /// This Rust API accepts a [`Plugin`] value. The JavaScript binding
    /// exposes the corresponding method as `use`.
    #[cfg(not(feature = "npm"))]
    pub fn use_plugin(&mut self, plugin: Plugin) {
        self.peisar_ast.add_plugin(plugin);
    }
    /// Renders the parsed Markdown as HTML using the configured render options.
    ///
    /// Registered visitors run before rendering.
    #[cfg_attr(feature = "npm", napi(getter))]
    pub fn html(&mut self) -> String {
        let doc = self.ast();
        let html = render_document_html(doc, Some(self.render_opts.clone()));
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
