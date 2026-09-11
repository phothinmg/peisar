//! Visitor pattern for walking the Markdown AST.
//!
//! Implement [`AstNodeVisitor`] and call [`visit_document`] (or the
//! `visit_*` methods directly) to traverse the tree.  The default
//! implementations of every method recurse into children, so you only
//! need to override the nodes you care about.

use crate::ast::*;

/// Trait for visiting AST nodes.
///
/// Each method receives a `&mut self` and a reference to the node.
/// The default implementations recurse into children — override only
/// the nodes you need.
pub trait AstNodeVisitor {
    // -- Document --

    fn visit_document(&mut self, doc: &Document) {
        self.visit_document_enter(doc);
        for block in &doc.children {
            self.visit_block(block);
        }
        self.visit_document_exit(doc);
    }

    fn visit_document_enter(&mut self, _doc: &Document) {}
    fn visit_document_exit(&mut self, _doc: &Document) {}

    // -- Blocks --

    fn visit_block(&mut self, block: &Block) {
        match block {
            Block::Heading {
                level,
                children,
                attrs,
                position: _,
            } => self.visit_heading(*level, children, attrs.as_ref()),
            Block::Paragraph {
                children,
                attrs,
                position: _,
            } => self.visit_paragraph(children, attrs.as_ref()),
            Block::CodeBlock {
                lang,
                code,
                attrs,
                position: _,
            } => self.visit_code_block(lang.as_deref(), code, attrs.as_ref()),
            Block::BlockQuote {
                children,
                attrs,
                position: _,
            } => self.visit_block_quote(children, attrs.as_ref()),
            Block::List {
                ordered,
                items,
                attrs,
                position: _,
            } => self.visit_list(*ordered, items, attrs.as_ref()),
            Block::ThematicBreak { attrs, position: _ } => {
                self.visit_thematic_break(attrs.as_ref())
            }
            Block::HtmlBlock { html, .. } => self.visit_html_block(html),
            Block::Table {
                table,
                attrs,
                position: _,
            } => self.visit_table(table, attrs.as_ref()),
        }
    }

    #[allow(unused)]
    fn visit_heading(
        &mut self,
        level: u8,
        children: &[Inline],
        attrs: Option<&KramdownAttributes>,
    ) {
        let _ = attrs;
        for inline in children {
            self.visit_inline(inline);
        }
    }

    fn visit_paragraph(&mut self, children: &[Inline], _attrs: Option<&KramdownAttributes>) {
        for inline in children {
            self.visit_inline(inline);
        }
    }

    fn visit_code_block(
        &mut self,
        _lang: Option<&str>,
        _code: &str,
        _attrs: Option<&KramdownAttributes>,
    ) {
    }

    fn visit_block_quote(&mut self, children: &[Block], _attrs: Option<&KramdownAttributes>) {
        for block in children {
            self.visit_block(block);
        }
    }

    fn visit_list(
        &mut self,
        _ordered: bool,
        items: &[ListItem],
        _attrs: Option<&KramdownAttributes>,
    ) {
        for item in items {
            self.visit_list_item(item);
        }
    }

    fn visit_list_item(&mut self, item: &ListItem) {
        for block in &item.children {
            self.visit_block(block);
        }
    }

    fn visit_thematic_break(&mut self, _attrs: Option<&KramdownAttributes>) {}

    fn visit_html_block(&mut self, _html: &str) {}

    fn visit_table(&mut self, table: &Table, _attrs: Option<&KramdownAttributes>) {
        self.visit_table_row(&table.header, &table.alignments, true);
        for row in &table.rows {
            self.visit_table_row(row, &table.alignments, false);
        }
    }

    fn visit_table_row(
        &mut self,
        row: &TableRow,
        _alignments: &[TableCellAlignment],
        _header: bool,
    ) {
        for cell in &row.cells {
            self.visit_table_cell(cell);
        }
    }

    fn visit_table_cell(&mut self, cell: &TableCell) {
        for inline in &cell.children {
            self.visit_inline(inline);
        }
    }

    // -- Inlines --

    fn visit_inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Text { value, .. } => self.visit_text(value),
            Inline::Emphasis {
                level,
                children,
                position: _,
            } => self.visit_emphasis(*level, children),
            Inline::Code { code, .. } => self.visit_inline_code(code),
            Inline::Link {
                text,
                url,
                title,
                autolink: _,
                position: _,
            } => self.visit_link(text, url, title.as_deref()),
            Inline::Image {
                alt,
                url,
                title,
                position: _,
            } => self.visit_image(alt, url, title.as_deref()),
            Inline::HardBreak { .. } => self.visit_hard_break(),
            Inline::SoftBreak { .. } => self.visit_soft_break(),
            Inline::HtmlInline { html, .. } => self.visit_inline_html(html),
            Inline::Strikethrough {
                children,
                position: _,
            } => self.visit_strikethrough(children),
        }
    }

    fn visit_text(&mut self, _text: &str) {}

    fn visit_emphasis(&mut self, _level: EmphasisLevel, children: &[Inline]) {
        for inline in children {
            self.visit_inline(inline);
        }
    }

    fn visit_inline_code(&mut self, _code: &str) {}

    fn visit_link(&mut self, text: &[Inline], _url: &str, _title: Option<&str>) {
        for inline in text {
            self.visit_inline(inline);
        }
    }

    fn visit_image(&mut self, _alt: &str, _url: &str, _title: Option<&str>) {}

    fn visit_hard_break(&mut self) {}

    fn visit_soft_break(&mut self) {}

    fn visit_inline_html(&mut self, _html: &str) {}

    fn visit_strikethrough(&mut self, children: &[Inline]) {
        for inline in children {
            self.visit_inline(inline);
        }
    }
}

/// Convenience function: walk `doc` with the given visitor.
pub fn visit_document<V: AstNodeVisitor + ?Sized>(visitor: &mut V, doc: &Document) {
    visitor.visit_document(doc);
}

// ---------------------------------------------------------------------------
// Example visitors
// ---------------------------------------------------------------------------

/// A visitor that counts various node types. Useful for testing and stats.
#[derive(Default, Debug)]
pub struct NodeCounter {
    pub headings: usize,
    pub paragraphs: usize,
    pub code_blocks: usize,
    pub block_quotes: usize,
    pub lists: usize,
    pub list_items: usize,
    pub thematic_breaks: usize,
    pub links: usize,
    pub images: usize,
    pub emphasis: usize,
    pub text_nodes: usize,
    pub inline_code: usize,
    pub tables: usize,
    pub strikethroughs: usize,
}

impl AstNodeVisitor for NodeCounter {
    fn visit_heading(
        &mut self,
        _level: u8,
        children: &[Inline],
        _attrs: Option<&KramdownAttributes>,
    ) {
        self.headings += 1;
        for inline in children {
            self.visit_inline(inline);
        }
    }
    fn visit_paragraph(&mut self, children: &[Inline], _attrs: Option<&KramdownAttributes>) {
        self.paragraphs += 1;
        for inline in children {
            self.visit_inline(inline);
        }
    }
    fn visit_code_block(
        &mut self,
        _lang: Option<&str>,
        _code: &str,
        _attrs: Option<&KramdownAttributes>,
    ) {
        self.code_blocks += 1;
    }
    fn visit_block_quote(&mut self, children: &[Block], _attrs: Option<&KramdownAttributes>) {
        self.block_quotes += 1;
        for block in children {
            self.visit_block(block);
        }
    }
    fn visit_list(
        &mut self,
        ordered: bool,
        items: &[ListItem],
        _attrs: Option<&KramdownAttributes>,
    ) {
        self.lists += 1;
        for item in items {
            self.visit_list_item(item);
        }
        let _ = ordered;
    }
    fn visit_list_item(&mut self, item: &ListItem) {
        self.list_items += 1;
        for block in &item.children {
            self.visit_block(block);
        }
    }
    fn visit_thematic_break(&mut self, _attrs: Option<&KramdownAttributes>) {
        self.thematic_breaks += 1;
    }
    fn visit_text(&mut self, _text: &str) {
        self.text_nodes += 1;
    }
    fn visit_emphasis(&mut self, level: EmphasisLevel, children: &[Inline]) {
        self.emphasis += 1;
        for inline in children {
            self.visit_inline(inline);
        }
        let _ = level;
    }
    fn visit_inline_code(&mut self, _code: &str) {
        self.inline_code += 1;
    }
    fn visit_link(&mut self, text: &[Inline], _url: &str, _title: Option<&str>) {
        self.links += 1;
        for inline in text {
            self.visit_inline(inline);
        }
    }
    fn visit_image(&mut self, _alt: &str, _url: &str, _title: Option<&str>) {
        self.images += 1;
    }
    fn visit_table(&mut self, table: &Table, _attrs: Option<&KramdownAttributes>) {
        self.tables += 1;
        self.visit_table_row(&table.header, &table.alignments, true);
        for row in &table.rows {
            self.visit_table_row(row, &table.alignments, false);
        }
    }
    fn visit_strikethrough(&mut self, children: &[Inline]) {
        self.strikethroughs += 1;
        for inline in children {
            self.visit_inline(inline);
        }
    }
}
