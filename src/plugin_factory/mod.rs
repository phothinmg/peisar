//! Plugin system for extending the parser and AST visitor.
//!
//! A plugin is a struct that implements [`Plugin`].  Plugins can:
//! - Transform the AST after parsing (via [`Plugin::transform_ast`]).
//! - Customize the HTML rendering pipeline (via [`Plugin::render_hooks`]).
//!
//! ## Creating a plugin
//!
//! ```rust,ignore
//! use peisar::plugin_factory::{Plugin, PluginContext, PluginResult};
//! use peisar::ast::Document;
//!
//! struct AutoLinkId;
//!
//! impl Plugin for AutoLinkId {
//!     fn name(&self) -> &str { "auto-link-id" }
//!
//!     fn transform_ast(&self, doc: &mut Document, _ctx: &PluginContext) {
//!         // Mutate the AST...
//!     }
//! }
//! ```
//!
//! ## Using the plugin pipeline
//!
//! ```rust,ignore
//! use peisar::plugin::{PluginPipeline, PluginContext};
//! use peisar::{parser, config::ParseOptions};
//!
//! let doc = parser::parse("# Hello");
//! let mut pipeline = PluginPipeline::new();
//! pipeline.add(AutoLinkId);
//! let ctx = PluginContext::new(ParseOptions::default());
//! let doc = pipeline.run(doc, &ctx);
//! ```

use crate::ast::Document;
use crate::config::ParseOptions;
use crate::html::AstToHtml;
// visitor trait used implicitly by AstToHtml

/// Context passed to plugins during processing.
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// The parse options that were used.
    pub options: ParseOptions,
}

impl PluginContext {
    pub fn new(options: ParseOptions) -> Self {
        Self { options }
    }
}

/// Result of running the plugin pipeline — the (possibly modified) document.
pub type PluginResult = Document;

/// Trait that every plugin must implement.
///
/// All methods have default no-op implementations, so a plugin only needs
/// to override the hooks it cares about.
pub trait Plugin: Send + Sync {
    /// Human-readable name (used for debugging / logging).
    fn name(&self) -> &str {
        "unnamed-plugin"
    }

    /// Transform the AST in-place after parsing.
    fn transform_ast(&self, _doc: &mut Document, _ctx: &PluginContext) {}

    /// Hook called before HTML rendering begins.
    fn pre_render(&self, _renderer: &mut AstToHtml, _doc: &Document, _ctx: &PluginContext) {}

    /// Hook called after HTML rendering finishes (before the output string
    /// is returned).
    fn post_render(&self, _html: &mut String, _doc: &Document, _ctx: &PluginContext) {}
}

/// A pipeline that runs plugins in order.
pub struct PluginPipeline {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginPipeline {
    /// Create an empty pipeline.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Add a plugin to the end of the pipeline.
    pub fn add<P: Plugin + 'static>(&mut self, plugin: P) -> &mut Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Run all plugins: AST transforms, then return the modified document.
    pub fn run_transforms(&self, mut doc: Document, ctx: &PluginContext) -> Document {
        for plugin in &self.plugins {
            plugin.transform_ast(&mut doc, ctx);
        }
        doc
    }

    /// Run pre-render hooks.
    pub fn run_pre_render(&self, renderer: &mut AstToHtml, doc: &Document, ctx: &PluginContext) {
        for plugin in &self.plugins {
            plugin.pre_render(renderer, doc, ctx);
        }
    }

    /// Run post-render hooks.
    pub fn run_post_render(&self, html: &mut String, doc: &Document, ctx: &PluginContext) {
        for plugin in &self.plugins {
            plugin.post_render(html, doc, ctx);
        }
    }

    /// Run the full pipeline: AST transforms → render → post-render.
    pub fn run(
        &self,
        doc: Document,
        ctx: &PluginContext,
        render_opts: &crate::config::RenderOptions,
    ) -> String {
        let doc = self.run_transforms(doc, ctx);
        let html = crate::html::render_document_with(&doc, render_opts, self, ctx);
        html
    }

    /// Iterate over plugin names.
    pub fn plugin_names(&self) -> impl Iterator<Item = &str> {
        self.plugins.iter().map(|p| p.name())
    }
}

impl Default for PluginPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Built-in plugins
// ---------------------------------------------------------------------------

/// A plugin that adds a class to every heading (e.g. `class="anchor"`).
pub struct HeadingClass {
    pub class: String,
}

impl Plugin for HeadingClass {
    fn name(&self) -> &str {
        "heading-class"
    }

    fn transform_ast(&self, doc: &mut Document, _ctx: &PluginContext) {
        use crate::ast::Block;
        use crate::ast::KramdownAttributes;
        for block in &mut doc.children {
            if let Block::Heading { attrs, .. } = block {
                let existing = attrs.take().unwrap_or_default();
                let mut classes = existing.classes;
                classes.push(self.class.clone());
                *attrs = Some(KramdownAttributes {
                    id: existing.id,
                    classes,
                    attributes: existing.attributes,
                });
            }
        }
    }
}

/// A plugin that wraps the document body in a `<div class="markdown-body">`.
/// (Applied as a post-render hook.)
pub struct WrapDiv {
    pub class: String,
}

impl Plugin for WrapDiv {
    fn name(&self) -> &str {
        "wrap-div"
    }

    fn post_render(&self, html: &mut String, _doc: &Document, _ctx: &PluginContext) {
        let wrapped = format!("<div class=\"{}\">\n{}</div>\n", self.class, html);
        *html = wrapped;
    }
}
