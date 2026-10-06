use crate::markdown::ast::tokens::token::{EmphasisLevel, TableCellAlignment, TaskState};
use crate::markdown::ast::{
    Document,
    tokens::{
        Attributes,
        token::{Block, Inline, ListItem, Table as AstTable},
    },
};
#[cfg(feature = "npm")]
use napi_derive::napi;
use serde::{Deserialize, Serialize};

/// Options that control how the AST is rendered to HTML.
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderOptions {
    /// If `true`, emit only the body content (no `<!DOCTYPE>`, `<html>`,
    /// `<head>`, or `<body>` wrapper).  If `false`, emit a full HTML
    /// document.  Default: `true` (fragment).
    pub fragment: bool,
    /// Include a `<meta charset="utf-8">` in the head (only relevant when
    /// `fragment` is `false`).  Default: `true`.
    pub charset: bool,
    /// Include a `<meta name="viewport" content="width=device-width,
    /// initial-scale=1.0">` in the head (only relevant when `fragment` is
    /// `false`).  Default: `true`.
    pub viewport: bool,
    /// Optional `<title>` for the HTML head (only relevant when
    /// `fragment` is `false`).  Default: `None`.
    pub title: Option<String>,
    /// Optional additional CSS classes to add to `<body>` (only relevant
    /// when `fragment` is `false`).  Default: `None`.
    pub body_class: Option<String>,
    /// Optional inline CSS to inject in a `<style>` tag in the head.
    /// Default: `None`.
    pub style: Option<String>,
}
#[cfg_attr(feature = "npm", napi)]
impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            fragment: true,
            charset: true,
            viewport: true,
            title: None,
            body_class: None,
            style: None,
        }
    }
}
/// Convenience: create `Some(true)` for fragment mode (backward-compatible
/// with the old `render_document(doc, Some(bool))` API).
#[cfg_attr(feature = "npm", napi)]
impl From<bool> for RenderOptions {
    fn from(fragment: bool) -> Self {
        Self {
            fragment,
            ..Default::default()
        }
    }
}

pub fn render_document_html(doc: &Document, opts: Option<RenderOptions>) -> String {
    let mut r = AstToHtml::new(opts);
    r.render_document(doc);
    r.finish()
}

pub struct AstToHtml {
    out: String,
    /// When `true`, inline text is HTML-escaped.
    escape: bool,
    opts: RenderOptions,
}

impl AstToHtml {
    /// Create a new renderer with the given options.
    ///
    /// Pass `None` to use [`RenderOptions::default`] (fragment mode).
    pub fn new(opts: Option<RenderOptions>) -> Self {
        let options = if opts.is_none() {
            RenderOptions::default()
        } else {
            opts.unwrap()
        };
        Self {
            out: String::new(),
            escape: true,
            opts: options,
        }
    }
    /// Consume the renderer and return the HTML string.
    pub fn finish(self) -> String {
        self.out
    }
    /// Render a [`Document`] and return the HTML string immediately
    /// (consuming the renderer).
    ///
    /// This is a convenience wrapper around [`render_document`](Self::render_document)
    /// + [`finish`](Self::finish).
    ///
    #[allow(unused)]
    pub fn render_document_owned(mut self, doc: &Document) -> String {
        self.render_document(doc);
        self.finish()
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
    fn emit_attrs(&mut self, attrs: Option<&Attributes>) {
        if let Some(a) = attrs {
            let s = a.to_html_attr_string();
            if !s.is_empty() {
                self.out.push(' ');
                self.out.push_str(&s);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Top-level entry
    // -----------------------------------------------------------------------

    /// Render a full [`Document`] into the internal buffer.
    ///
    /// When `RenderOptions::fragment` is `true` (the default) only the body
    /// content is emitted.  When `false`, a complete `<!DOCTYPE html>`
    /// document with `<html>`, `<head>`, and `<body>` wrappers is produced.
    pub fn render_document(&mut self, doc: &Document) {
        if self.opts.fragment {
            for b in &doc.children {
                self.render_block(b);
            }
        } else {
            self.out.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
            if self.opts.charset {
                self.out.push_str("<meta charset=\"utf-8\">\n");
            }
            if self.opts.viewport {
                self.out.push_str(
                    "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n",
                );
            }
            if let Some(title) = self.opts.title.clone() {
                self.out.push_str("<title>");
                self.esc(&title);
                self.out.push_str("</title>\n");
            }
            if let Some(style) = self.opts.style.clone() {
                self.out.push_str("<style>\n");
                self.out.push_str(&style);
                self.out.push_str("\n</style>\n");
            }
            self.out.push_str("</head>\n<body");
            if let Some(bc) = self.opts.body_class.clone() {
                self.out.push_str(" class=\"");
                self.esc_no_quote(&bc);
                self.out.push_str("\"");
            }
            self.out.push_str(">\n");
            for b in &doc.children {
                self.render_block(b);
            }
            self.out.push_str("</body>\n</html>\n");
        }
    }

    // -----------------------------------------------------------------------
    // Blocks
    // -----------------------------------------------------------------------

    /// Render a single block-level node.
    pub fn render_block(&mut self, block: &Block) {
        match block {
            Block::Heading {
                level,
                children,
                attrs,
                ..
            } => {
                self.out.push_str(&format!("<h{}", level));
                self.emit_attrs(attrs.as_ref());
                self.out.push('>');
                for inline in children {
                    self.render_inline(inline);
                }
                self.out.push_str(&format!("</h{}>\n", level));
            }
            Block::Paragraph {
                children, attrs, ..
            } => {
                self.out.push_str("<p");
                self.emit_attrs(attrs.as_ref());
                self.out.push('>');
                for inline in children {
                    self.render_inline(inline);
                }
                self.out.push_str("</p>\n");
            }
            Block::CodeBlock {
                lang, code, attrs, ..
            } => {
                self.out.push_str("<pre><code");
                if let Some(l) = lang {
                    if !l.is_empty() {
                        self.out.push_str(" class=\"language-");
                        self.esc_no_quote(l);
                        self.out.push_str("\"");
                    }
                }
                self.emit_attrs(attrs.as_ref());
                self.out.push('>');
                // Code blocks are never escaped for *content* when we want
                // raw output, but we must escape < > & for valid HTML.
                self.esc(code);
                self.out.push_str("</code></pre>\n");
            }
            Block::BlockQuote {
                children, attrs, ..
            } => {
                self.out.push_str("<blockquote");
                self.emit_attrs(attrs.as_ref());
                self.out.push_str(">\n");
                for b in children {
                    self.render_block(b);
                }
                self.out.push_str("</blockquote>\n");
            }
            Block::List {
                ordered,
                items,
                attrs,
                ..
            } => {
                let tag = if *ordered { "ol" } else { "ul" };
                self.out.push('<');
                self.out.push_str(tag);
                self.emit_attrs(attrs.as_ref());
                self.out.push_str(">\n");
                for item in items {
                    self.render_list_item(item);
                }
                self.out.push_str(&format!("</{}>\n", tag));
            }
            Block::ThematicBreak { .. } => {
                self.out.push_str("<hr>\n");
            }
            Block::HtmlBlock { html, .. } => {
                self.out.push_str(html);
                if !html.ends_with('\n') {
                    self.out.push('\n');
                }
            }
            Block::Table { table, attrs, .. } => {
                self.render_table(table, attrs);
            }
            Block::LinkReferenceDefinition { .. } => {
                // Not rendered as visible HTML.
            }
            Block::Comment { value, .. } => {
                self.out.push_str("<!-- ");
                self.out.push_str(value);
                self.out.push_str(" -->\n");
            }
        }
    }

    /// Render a single list item (`<li>`).
    fn render_list_item(&mut self, item: &ListItem) {
        self.out.push_str("<li");
        if let Some(task) = &item.task {
            let checked = matches!(task, TaskState::Checked);
            self.out.push_str(" class=\"task-list-item\"");
            if checked {
                self.out.push_str(" data-checked=\"true\"");
            }
        }
        self.out.push('>');
        if let Some(task) = &item.task {
            let checked = matches!(task, TaskState::Checked);
            self.out
                .push_str("<input type=\"checkbox\" class=\"task-list-item-checkbox\"");
            if checked {
                self.out.push_str(" checked");
            }
            self.out.push_str(" disabled> ");
        }
        // Render nested blocks.  If the item contains a single paragraph we
        // render it inline (no wrapping <p>) per CommonMark rendering
        // convention for tight lists.
        let tight = item.children.len() == 1
            && matches!(item.children.first(), Some(Block::Paragraph { .. }));
        if tight {
            if let Some(Block::Paragraph { children, .. }) = item.children.first() {
                for inline in children {
                    self.render_inline(inline);
                }
            }
        } else {
            self.out.push('\n');
            for b in &item.children {
                self.render_block(b);
            }
        }
        self.out.push_str("</li>\n");
    }

    /// Render a GFM table.
    fn render_table(&mut self, table: &AstTable, attrs: &Option<Attributes>) {
        self.out.push_str("<table");
        self.emit_attrs(attrs.as_ref());
        self.out.push_str(">\n<thead>\n<tr>\n");
        // Header
        for (i, cell) in table.header.cells.iter().enumerate() {
            self.out.push_str("<th");
            self.emit_align(table.alignments.get(i));
            self.out.push('>');
            for inline in &cell.children {
                self.render_inline(inline);
            }
            self.out.push_str("</th>\n");
        }
        self.out.push_str("</tr>\n</thead>\n<tbody>\n");
        // Body rows
        for row in &table.rows {
            self.out.push_str("<tr>\n");
            for (i, cell) in row.cells.iter().enumerate() {
                self.out.push_str("<td");
                self.emit_align(table.alignments.get(i));
                self.out.push('>');
                for inline in &cell.children {
                    self.render_inline(inline);
                }
                self.out.push_str("</td>\n");
            }
            self.out.push_str("</tr>\n");
        }
        self.out.push_str("</tbody>\n</table>\n");
    }

    /// Emit a `style="text-align: …"` attribute for a table column alignment.
    fn emit_align(&mut self, align: Option<&TableCellAlignment>) {
        if let Some(a) = align {
            let css = match a {
                TableCellAlignment::Default | TableCellAlignment::Left => return,
                TableCellAlignment::Center => "text-align: center",
                TableCellAlignment::Right => "text-align: right",
            };
            self.out.push_str(" style=\"");
            self.out.push_str(css);
            self.out.push('"');
        }
    }

    // -----------------------------------------------------------------------
    // Inlines
    // -----------------------------------------------------------------------

    /// Render a single inline-level node.
    pub fn render_inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Text { value, .. } => {
                self.esc(value);
            }
            Inline::Emphasis {
                level, children, ..
            } => {
                let (open, close) = match level {
                    EmphasisLevel::Italic => ("<em>", "</em>"),
                    EmphasisLevel::Bold => ("<strong>", "</strong>"),
                };
                self.out.push_str(open);
                for child in children {
                    self.render_inline(child);
                }
                self.out.push_str(close);
            }
            Inline::Code { code, .. } => {
                self.out.push_str("<code>");
                self.esc(code);
                self.out.push_str("</code>");
            }
            Inline::HtmlInline { html, .. } => {
                // Raw HTML — emit as-is.
                self.out.push_str(html);
            }
            Inline::Strikethrough { children, .. } => {
                self.out.push_str("<del>");
                for child in children {
                    self.render_inline(child);
                }
                self.out.push_str("</del>");
            }
            Inline::HardBreak { .. } => {
                self.out.push_str("<br>\n");
            }
            Inline::SoftBreak { .. } => {
                self.out.push('\n');
            }
            Inline::Image {
                alt, url, title, ..
            } => {
                self.out.push_str("<img src=\"");
                self.esc_attr(url);
                self.out.push_str("\" alt=\"");
                self.esc(alt);
                self.out.push('"');
                if let Some(t) = title {
                    self.out.push_str(" title=\"");
                    self.esc(t);
                    self.out.push('"');
                }
                self.out.push('>');
            }
            Inline::Link {
                text, url, title, ..
            } => {
                self.out.push_str("<a href=\"");
                self.esc_attr(url);
                self.out.push('"');
                if let Some(t) = title {
                    self.out.push_str(" title=\"");
                    self.esc(t);
                    self.out.push('"');
                }
                self.out.push('>');
                for child in text {
                    self.render_inline(child);
                }
                self.out.push_str("</a>");
            }
            Inline::LinkReference {
                text, url, title, ..
            } => {
                self.out.push_str("<a href=\"");
                self.esc_attr(url);
                self.out.push('"');
                if let Some(t) = title {
                    self.out.push_str(" title=\"");
                    self.esc(t);
                    self.out.push('"');
                }
                self.out.push('>');
                for child in text {
                    self.render_inline(child);
                }
                self.out.push_str("</a>");
            }
        }
    }

    // -----------------------------------------------------------------------
    // Small helpers
    // -----------------------------------------------------------------------

    /// Escape text for use inside an HTML attribute value (escapes `&`, `"`,
    /// `<`, `>`).
    fn esc_attr(&mut self, text: &str) {
        for c in text.chars() {
            match c {
                '&' => self.out.push_str("&amp;"),
                '"' => self.out.push_str("&quot;"),
                '<' => self.out.push_str("&lt;"),
                '>' => self.out.push_str("&gt;"),
                _ => self.out.push(c),
            }
        }
    }

    /// Escape text without escaping single quotes — used for class names and
    /// other attribute values that won't contain `'`.
    fn esc_no_quote(&mut self, text: &str) {
        for c in text.chars() {
            match c {
                '&' => self.out.push_str("&amp;"),
                '"' => self.out.push_str("&quot;"),
                '<' => self.out.push_str("&lt;"),
                '>' => self.out.push_str("&gt;"),
                _ => self.out.push(c),
            }
        }
    }
}

impl Default for AstToHtml {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderOptions, render_document_html};
    use crate::markdown::ast::{AstOptions, Document};

    #[test]
    fn body_class_is_not_written_to_html_element() {
        let doc = Document::parse("Text", &AstOptions::default(), None);
        let html = render_document_html(
            &doc,
            Some(RenderOptions {
                fragment: false,
                body_class: Some("markdown-body".to_string()),
                ..RenderOptions::default()
            }),
        );

        assert!(html.contains("<html>\n<head>"));
        assert!(html.contains("<body class=\"markdown-body\">"));
        assert!(!html.contains("<html class="));
    }

    /// Renders a document as a fragment and strips the trailing newline so
    /// assertions read cleanly.
    fn frag(md: &str) -> String {
        let doc = Document::parse(md, &AstOptions::default(), None);
        render_document_html(&doc, Some(RenderOptions::default()))
    }

    #[test]
    fn default_options_render_a_fragment() {
        // RenderOptions::default() has fragment = true.
        let html = frag("# Hi\n");
        assert_eq!(html, "<h1>Hi</h1>\n");
    }

    #[test]
    fn full_document_wrapper() {
        let doc = Document::parse("Hi\n", &AstOptions::default(), None);
        let html = render_document_html(
            &doc,
            Some(RenderOptions {
                fragment: false,
                title: Some("Page".to_string()),
                style: Some("p{margin:0}".to_string()),
                ..RenderOptions::default()
            }),
        );
        assert!(html.starts_with("<!DOCTYPE html>\n<html>\n<head>"));
        assert!(html.contains("<meta charset=\"utf-8\">"));
        assert!(html.contains("<meta name=\"viewport\""));
        assert!(html.contains("<title>Page</title>"));
        assert!(html.contains("<style>\np{margin:0}\n</style>"));
        assert!(html.ends_with("</body>\n</html>\n"));
    }

    #[test]
    fn full_document_without_charset_and_viewport() {
        let doc = Document::parse("Hi\n", &AstOptions::default(), None);
        let html = render_document_html(
            &doc,
            Some(RenderOptions {
                fragment: false,
                charset: false,
                viewport: false,
                ..RenderOptions::default()
            }),
        );
        assert!(!html.contains("meta charset"));
        assert!(!html.contains("viewport"));
    }

    #[test]
    fn title_is_escaped() {
        let doc = Document::parse("Hi\n", &AstOptions::default(), None);
        let html = render_document_html(
            &doc,
            Some(RenderOptions {
                fragment: false,
                title: Some("<script>".to_string()),
                ..RenderOptions::default()
            }),
        );
        assert!(html.contains("<title>&lt;script&gt;</title>"));
    }

    #[test]
    fn blocks_render_to_expected_html() {
        assert_eq!(frag("# Title\n"), "<h1>Title</h1>\n");
        assert_eq!(frag("## Sub\n"), "<h2>Sub</h2>\n");
        assert_eq!(frag("Para\n"), "<p>Para</p>\n");
        assert_eq!(
            frag("```rust\nfn f()\n```\n"),
            "<pre><code class=\"language-rust\">fn f()</code></pre>\n"
        );
        assert_eq!(
            frag("> quote\n"),
            "<blockquote>\n<p>quote</p>\n</blockquote>\n"
        );
        assert_eq!(frag("---\n"), "<hr>\n");
        assert_eq!(frag("- a\n- b\n"), "<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n");
        assert_eq!(frag("1. a\n"), "<ol>\n<li>a</li>\n</ol>\n");
        assert_eq!(frag("<!-- hi -->\n"), "<!-- hi -->\n");
    }

    #[test]
    fn kramdown_attributes_are_emitted() {
        let html = frag("# Title\n{: #hero .card}\n");
        assert_eq!(html, "<h1 id=\"hero\" class=\"card\">Title</h1>\n");
    }

    #[test]
    fn code_block_content_is_escaped() {
        let html = frag("```\nif a < b && c > d\n```\n");
        assert!(html.contains("if a &lt; b &amp;&amp; c &gt; d"));
    }

    #[test]
    fn tight_list_items_skip_paragraph_tags() {
        // A single-paragraph item renders inline inside <li> (tight list).
        let html = frag("- one\n- two\n");
        assert_eq!(html, "<ul>\n<li>one</li>\n<li>two</li>\n</ul>\n");
        // Multi-block items keep <p> wrappers.
        let html = frag("- a\n\n  b\n");
        assert!(html.contains("<li>\n<p>a</p>\n"));
    }

    #[test]
    fn task_lists_render_checkboxes() {
        let html = frag("- [ ] todo\n- [x] done\n");
        assert!(html.contains("class=\"task-list-item\""));
        assert!(html.contains("data-checked=\"true\""));
        assert!(html.contains("<input type=\"checkbox\""));
        // The marker itself must be stripped — only the label text renders.
        assert!(!html.contains("[ ]"));
        assert!(!html.contains("[x]"));
        assert!(html.contains("disabled> todo</li>"));
        assert!(html.contains("checked disabled> done</li>"));
        // Only the checked item carries the `checked` attribute.
        assert!(html.contains("checked disabled"));
    }

    #[test]
    fn tables_render_alignments() {
        let html = frag("| a | b | c |\n| :- | :-: | -: |\n| 1 | 2 | 3 |\n");
        assert!(html.contains("<th>"));
        assert!(html.contains("<td style=\"text-align: center\">2"));
        assert!(html.contains("<td style=\"text-align: right\">3"));
    }

    #[test]
    fn link_reference_definitions_render_nothing() {
        let html = frag("[ref]: https://example.com\n");
        assert_eq!(html, "");
    }

    #[test]
    fn inlines_render_to_expected_html() {
        assert_eq!(frag("*em*\n"), "<p><em>em</em></p>\n");
        assert_eq!(frag("**bold**\n"), "<p><strong>bold</strong></p>\n");
        assert_eq!(frag("`x`\n"), "<p><code>x</code></p>\n");
        assert_eq!(frag("~~gone~~\n"), "<p><del>gone</del></p>\n");
        assert_eq!(
            frag("[t](https://e.com)\n"),
            "<p><a href=\"https://e.com\">t</a></p>\n"
        );
        assert_eq!(
            frag("![alt](https://e.com/i.png)\n"),
            "<p><img src=\"https://e.com/i.png\" alt=\"alt\"></p>\n"
        );
        // Soft break → newline, hard break → <br>.
        assert_eq!(frag("a\nb\n"), "<p>a\nb</p>\n");
        assert_eq!(frag("a  \nb\n"), "<p>a<br>\nb</p>\n");
    }

    #[test]
    fn urls_are_escaped_in_attributes() {
        // `&` in a URL must become &amp; inside the href attribute.
        let html = frag("[t](https://e.com/?a=1&b=2)\n");
        assert!(html.contains("href=\"https://e.com/?a=1&amp;b=2\""));
    }

    #[test]
    fn html_blocks_pass_through_unescaped() {
        let html = frag("<div class=\"x\">raw</div>\n");
        assert!(html.contains("<div class=\"x\">raw</div>\n"));
    }

    #[test]
    fn inline_html_passes_through_unescaped() {
        let html = frag("a <span>b</span>\n");
        assert_eq!(html, "<p>a <span>b</span></p>\n");
    }

    #[test]
    fn text_is_escaped() {
        // `<Chips>` would be parsed as inline HTML, so escape-test with
        // characters that stay inside a Text node.
        let html = frag("Fish & Chips > \"q\"\n");
        assert!(html.contains("Fish &amp; Chips &gt; &quot;q&quot;"));
        // `&` and quotes are also escaped inside code spans.
        let html = frag("`a & b`\n");
        assert!(html.contains("<code>a &amp; b</code>"));
    }

    #[test]
    fn render_document_owned_is_a_convenience_wrapper() {
        let doc = Document::parse("Hi\n", &AstOptions::default(), None);
        let html = super::AstToHtml::new(None).render_document_owned(&doc);
        assert_eq!(html, "<p>Hi</p>\n");
    }
}
