//! AST → HTML renderer.
//!
//! [`AstToHtml`] implements [`crate::visitor::AstNodeVisitor`] and emits an
//! HTML string.  Call [`render_document`] for the full pipeline, or use the
//! visitor directly if you want to integrate it into a larger walk.
//!
//! Rendering is controlled by [`crate::config::RenderOptions`] which
//! determines whether the output is a full HTML document or just a
//! fragment.

use crate::ast::*;
use crate::config::RenderOptions;
use crate::plugin_factory::{PluginContext, PluginPipeline};
use crate::visitor::AstNodeVisitor;

/// Render a [`Document`] into an HTML string using default render options
/// (fragment mode).
pub fn render_document(doc: &Document, fragment: Option<bool>) -> String {
    let opts = match fragment {
        Some(f) => RenderOptions {
            fragment: f,
            ..Default::default()
        },
        None => RenderOptions::default(),
    };
    render_document_with(
        doc,
        &opts,
        &PluginPipeline::new(),
        &PluginContext::new(crate::config::ParseOptions::default()),
    )
}

/// Render a [`Document`] into an HTML string with full options and plugin
/// pipeline.  Runs AST transform plugins, then renders, then runs
/// post-render plugins.
pub fn render_document_with(
    doc: &Document,
    opts: &RenderOptions,
    pipeline: &PluginPipeline,
    ctx: &PluginContext,
) -> String {
    let doc = doc.clone();
    let doc = pipeline.run_transforms(doc, ctx);
    let mut renderer = AstToHtml::with_options(opts.clone());
    pipeline.run_pre_render(&mut renderer, &doc, ctx);
    renderer.visit_document(&doc);
    let mut html = renderer.finish();
    pipeline.run_post_render(&mut html, &doc, ctx);
    html
}

/// A visitor that builds an HTML string from the AST.
pub struct AstToHtml {
    out: String,
    /// When `true`, inline text is HTML-escaped.
    escape: bool,
    opts: RenderOptions,
}

impl AstToHtml {
    pub fn new() -> Self {
        Self {
            out: String::new(),
            escape: true,
            opts: RenderOptions::default(),
        }
    }

    /// Create a renderer with the given options.
    pub fn with_options(opts: RenderOptions) -> Self {
        Self {
            out: String::new(),
            escape: true,
            opts,
        }
    }

    /// Consume the renderer and return the HTML string.
    pub fn finish(self) -> String {
        self.out
    }

    fn esc(&mut self, text: &str) {
        if self.escape {
            for c in text.chars() {
                match c {
                    '&' => self.out.push_str("&amp;"),
                    '<' => self.out.push_str("&lt;"),
                    '>' => self.out.push_str("&gt;"),
                    '"' => self.out.push_str("&quot;"),
                    '\'' => self.out.push_str("&#39;"),
                    _ => self.out.push(c),
                }
            }
        } else {
            self.out.push_str(text);
        }
    }

    /// Emit Kramdown attributes as an HTML attribute string (with a leading
    /// space if non-empty).
    fn emit_attrs(&mut self, attrs: Option<&KramdownAttributes>) {
        if let Some(a) = attrs {
            if !a.is_empty() {
                self.out.push(' ');
                self.out.push_str(&a.to_html_attr_string());
            }
        }
    }
}

impl Default for AstToHtml {
    fn default() -> Self {
        Self::new()
    }
}

impl AstNodeVisitor for AstToHtml {
    fn visit_document_enter(&mut self, _doc: &Document) {
        if !self.opts.fragment {
            self.out.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
            if self.opts.charset {
                self.out.push_str("<meta charset=\"utf-8\">\n");
            }
            if self.opts.viewport {
                self.out.push_str(
                    "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n",
                );
            }
            if let Some(ref title) = self.opts.title {
                self.out.push_str(&format!("<title>{}</title>\n", title));
            }
            if let Some(ref style) = self.opts.style {
                self.out.push_str("<style>\n");
                self.out.push_str(style);
                self.out.push_str("\n</style>\n");
            }
            self.out.push_str("</head>\n<body");
            if let Some(ref cls) = self.opts.body_class {
                self.out.push_str(&format!(" class=\"{}\"", cls));
            }
            self.out.push_str(">\n");
        }
    }

    fn visit_document_exit(&mut self, _doc: &Document) {
        if !self.opts.fragment {
            self.out.push_str("</body>\n</html>\n");
        }
    }

    fn visit_heading(
        &mut self,
        level: u8,
        children: &[Inline],
        attrs: Option<&KramdownAttributes>,
    ) {
        self.out.push_str(&format!("<h{}", level));
        self.emit_attrs(attrs);
        self.out.push('>');
        for inline in children {
            self.visit_inline(inline);
        }
        self.out.push_str(&format!("</h{}>\n", level));
    }

    fn visit_paragraph(&mut self, children: &[Inline], attrs: Option<&KramdownAttributes>) {
        self.out.push_str("<p");
        self.emit_attrs(attrs);
        self.out.push('>');
        for inline in children {
            self.visit_inline(inline);
        }
        self.out.push_str("</p>\n");
    }

    fn visit_code_block(
        &mut self,
        lang: Option<&str>,
        code: &str,
        attrs: Option<&KramdownAttributes>,
    ) {
        self.out.push_str("<pre><code");
        if let Some(l) = lang {
            self.out.push_str(&format!(" class=\"language-{}\"", l));
        }
        self.emit_attrs(attrs);
        self.out.push('>');
        self.escape = true;
        self.esc(code);
        self.out.push_str("</code></pre>\n");
    }

    fn visit_block_quote(&mut self, children: &[Block], attrs: Option<&KramdownAttributes>) {
        self.out.push_str("<blockquote");
        self.emit_attrs(attrs);
        self.out.push_str(">\n");
        for block in children {
            self.visit_block(block);
        }
        self.out.push_str("</blockquote>\n");
    }

    fn visit_list(
        &mut self,
        ordered: bool,
        items: &[ListItem],
        attrs: Option<&KramdownAttributes>,
    ) {
        let tag = if ordered { "ol" } else { "ul" };
        self.out.push_str(&format!("<{}", tag));
        self.emit_attrs(attrs);
        self.out.push_str(">\n");
        for item in items {
            self.visit_list_item(item);
        }
        self.out.push_str(&format!("</{}>\n", tag));
    }

    fn visit_list_item(&mut self, item: &ListItem) {
        self.out.push_str("<li>");
        if let Some(task) = item.task {
            let checked = matches!(task, TaskState::Checked);
            self.out.push_str(&format!(
                "<input type=\"checkbox\"{} disabled /> ",
                if checked { " checked" } else { "" }
            ));
        }
        if item.children.len() == 1 {
            if let Block::Paragraph { children, .. } = &item.children[0] {
                for inline in children {
                    self.visit_inline(inline);
                }
                self.out.push_str("</li>\n");
                return;
            }
        }
        for block in &item.children {
            self.visit_block(block);
        }
        self.out.push_str("</li>\n");
    }

    fn visit_thematic_break(&mut self, attrs: Option<&KramdownAttributes>) {
        self.out.push_str("<hr");
        self.emit_attrs(attrs);
        self.out.push_str(" />\n");
    }

    fn visit_html_block(&mut self, html: &str) {
        self.out.push_str(html);
        self.out.push('\n');
    }

    // -- Tables (GFM) --

    fn visit_table(&mut self, table: &Table, attrs: Option<&KramdownAttributes>) {
        self.out.push_str("<table");
        self.emit_attrs(attrs);
        self.out.push_str(">\n");

        // Header
        self.out.push_str("<thead>\n<tr>\n");
        for (i, cell) in table.header.cells.iter().enumerate() {
            let align = table.alignments.get(i).copied().unwrap_or_default();
            let style = match align {
                TableCellAlignment::Left => " style=\"text-align: left\"",
                TableCellAlignment::Center => " style=\"text-align: center\"",
                TableCellAlignment::Right => " style=\"text-align: right\"",
                TableCellAlignment::Default => "",
            };
            self.out.push_str(&format!("<th{}>", style));
            for inline in &cell.children {
                self.visit_inline(inline);
            }
            self.out.push_str("</th>\n");
        }
        self.out.push_str("</tr>\n</thead>\n");

        // Body
        self.out.push_str("<tbody>\n");
        for row in &table.rows {
            self.out.push_str("<tr>\n");
            for (i, cell) in row.cells.iter().enumerate() {
                let align = table.alignments.get(i).copied().unwrap_or_default();
                let style = match align {
                    TableCellAlignment::Left => " style=\"text-align: left\"",
                    TableCellAlignment::Center => " style=\"text-align: center\"",
                    TableCellAlignment::Right => " style=\"text-align: right\"",
                    TableCellAlignment::Default => "",
                };
                self.out.push_str(&format!("<td{}>", style));
                for inline in &cell.children {
                    self.visit_inline(inline);
                }
                self.out.push_str("</td>\n");
            }
            self.out.push_str("</tr>\n");
        }
        self.out.push_str("</tbody>\n");
        self.out.push_str("</table>\n");
    }

    // -- In_lines --

    fn visit_text(&mut self, text: &str) {
        self.esc(text);
    }

    fn visit_emphasis(&mut self, level: EmphasisLevel, children: &[Inline]) {
        let tag = match level {
            EmphasisLevel::Italic => "em",
            EmphasisLevel::Bold => "strong",
        };
        self.out.push_str(&format!("<{}>", tag));
        for inline in children {
            self.visit_inline(inline);
        }
        self.out.push_str(&format!("</{}>", tag));
    }

    fn visit_inline_code(&mut self, code: &str) {
        self.out.push_str("<code>");
        self.esc(code);
        self.out.push_str("</code>");
    }

    fn visit_link(&mut self, text: &[Inline], url: &str, title: Option<&str>) {
        self.out.push_str("<a href=\"");
        self.esc(url);
        if let Some(t) = title {
            self.out.push_str("\" title=\"");
            self.esc(t);
        }
        self.out.push_str("\">");
        for inline in text {
            self.visit_inline(inline);
        }
        self.out.push_str("</a>");
    }

    fn visit_image(&mut self, alt: &str, url: &str, title: Option<&str>) {
        self.out.push_str("<img src=\"");
        self.esc(url);
        self.out.push_str("\" alt=\"");
        self.esc(alt);
        if let Some(t) = title {
            self.out.push_str("\" title=\"");
            self.esc(t);
        }
        self.out.push_str("\" />");
    }

    fn visit_hard_break(&mut self) {
        self.out.push_str("<br />\n");
    }

    fn visit_soft_break(&mut self) {
        self.out.push('\n');
    }

    fn visit_inline_html(&mut self, html: &str) {
        self.out.push_str(html);
    }

    fn visit_strikethrough(&mut self, children: &[Inline]) {
        self.out.push_str("<del>");
        for inline in children {
            self.visit_inline(inline);
        }
        self.out.push_str("</del>");
    }
}
