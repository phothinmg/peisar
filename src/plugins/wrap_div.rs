use crate::ast::Document;
use crate::plugin_factory::{Plugin, PluginContext};

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
