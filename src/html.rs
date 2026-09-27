use crate::ast::{
    Document,
    tokens::{
        Attributes,
        token::{
            Block, EmphasisLevel, Inline, ListItem, Table as AstTable, TableCellAlignment,
            TaskState,
        },
    },
};
use napi_derive::napi;
use serde::{Deserialize, Serialize};

/// Options that control how the AST is rendered to HTML.
#[napi(object)]
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
#[napi]
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
#[napi]
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
    /// # Example
    ///
    /// ```rust
    /// use peisar_ast::{Document, AstOptions};
    /// use peisar_html::AstToHtml;
    ///
    /// let doc = Document::parse("Hi.\n", &AstOptions::default(), None);
    /// let html = AstToHtml::default().render_document_owned(&doc);
    /// assert_eq!(html, "<p>Hi.</p>\n");
    /// ```
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
            self.out.push_str("<!DOCTYPE html>\n<html");
            if let Some(bc) = self.opts.body_class.clone() {
                self.out.push_str(" class=\"");
                self.esc_no_quote(&bc);
                self.out.push_str("\"");
            }
            self.out.push_str(">\n<head>\n");
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
