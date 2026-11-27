use crate::ast::{
    span::Span,
    tokens::{Attributes, Block, Inline},
};
#[cfg(feature = "npm")]
use napi_derive::napi;

/// Unified plugin factory for the AST engine.
///
/// One trait covers both extension phases — parsing hooks (`try_parse_block` /
/// `try_parse_inline`) and visitor callbacks (`visit_block` / `visit_inline`).
/// All methods have default no-op / declining implementations — implement
/// only the callbacks a plugin needs.
pub trait PluginFactory {
    /// Try to parse a block starting at the current source line.
    ///
    /// Return the parsed block and number of lines consumed, or `None` to
    /// decline this position.
    fn try_parse_block(&self, _ctx: &BlockParserContext) -> Option<(Block, usize)> {
        None
    }

    /// Try to parse inline content starting at the current character.
    ///
    /// Return the parsed inline node and number of characters consumed, or
    /// `None` to decline this position.
    fn try_parse_inline(&self, _ctx: &InlineParserContext) -> Option<(Inline, usize)> {
        None
    }

    /// Visit a block node and return the edits to apply to it.
    fn visit_block(&mut self, _block: &mut Block) -> VisitControl {
        VisitControl::default()
    }

    /// Visit an inline node and return the edits to apply to it.
    fn visit_inline(&mut self, _inline: &mut Inline) -> InlineVisitControl {
        InlineVisitControl::default()
    }
    /// Whether this factory carries parse hooks (i.e. the host must
    /// re-parse the raw source for its effects to show).  Default `false`
    /// — the JS adapter overrides this when `parseBlock` / `parseInline`
    /// callbacks are registered.
    fn has_parse_hooks(&self) -> bool {
        false
    }
}

/// Context passed to block parser hooks.
#[cfg_attr(feature = "npm", napi(object, namespace = "peisar"))]
#[derive(Debug, Clone)]
pub struct BlockParserContext {
    /// The current (unmodified) source line.
    pub line: String,
    /// 0-based index of the current line in the (sub-)document.
    pub line_index: u32,
    /// Remaining lines starting at the current line (`lines[0]` is `line`).
    pub lines: Vec<String>,
}

/// Context passed to inline parser hooks.
///
/// On the JavaScript side the fields are exposed as `rest` and `index`.
#[cfg_attr(feature = "npm", napi(object, namespace = "peisar"))]
#[derive(Debug, Clone)]
pub struct InlineParserContext {
    /// Remaining inline text starting at the current character position.
    pub rest: String,
    /// 0-based character index of the current position in the inline text.
    pub index: u32,
}
/// Control returned from `visit_block` describing edits to perform.
///
/// Returned by [`PluginFactory::visit_block`].  Mutually exclusive fields are
/// resolved in the order: `remove` → `replace_with` → keep.  `insert_before`
/// and `insert_after` apply regardless.
#[derive(Debug, Default)]
pub struct VisitControl {
    /// Nodes to insert before the current node's position.
    pub insert_before: Vec<Block>,
    /// Nodes to insert after the current node's position.
    pub insert_after: Vec<Block>,
    /// Replace the current node with these nodes.
    pub replace_with: Option<Vec<Block>>,
    /// Remove the current node entirely.
    pub remove: bool,
    /// Whether to recurse into this node's child nodes (when applicable).
    /// Default `false`.
    pub recurse: bool,
}

impl VisitControl {
    /// Convenience: keep the (possibly mutated) node and recurse into children.
    pub fn keep_and_recurse() -> Self {
        VisitControl {
            recurse: true,
            ..Default::default()
        }
    }

    /// Convenience: remove the node.
    pub fn remove() -> Self {
        VisitControl {
            remove: true,
            ..Default::default()
        }
    }

    /// Convenience: replace current node with given nodes.
    ///
    /// The replacements are not themselves visited.
    pub fn replace_with(nodes: Vec<Block>) -> Self {
        VisitControl {
            replace_with: Some(nodes),
            ..Default::default()
        }
    }
}

/// Control returned from `visit_inline` describing edits to perform on inline
/// nodes. Mirrors `VisitControl` but for `Inline` nodes.
#[derive(Debug, Default)]
pub struct InlineVisitControl {
    /// Nodes to insert before the current node's position.
    pub insert_before: Vec<Inline>,
    /// Nodes to insert after the current node's position.
    pub insert_after: Vec<Inline>,
    /// Replace the current node with these nodes.
    pub replace_with: Option<Vec<Inline>>,
    /// Remove the current node entirely.
    pub remove: bool,
    /// Whether to recurse into this node's child nodes (when applicable).
    pub recurse: bool,
}

impl InlineVisitControl {
    /// Convenience: keep the (possibly mutated) node and recurse into children.
    pub fn keep_and_recurse() -> Self {
        InlineVisitControl {
            recurse: true,
            ..Default::default()
        }
    }
    /// Convenience: remove the node.
    pub fn remove() -> Self {
        InlineVisitControl {
            remove: true,
            ..Default::default()
        }
    }
    /// Convenience: replace current node with given nodes.
    pub fn replace_with(nodes: Vec<Inline>) -> Self {
        InlineVisitControl {
            replace_with: Some(nodes),
            ..Default::default()
        }
    }
}

// 1 ============
/// Visit and possibly mutate a whole document. Traversal order is pre-order
/// (visitor is called before recursing into children). The visitor may:
/// - mutate the node in-place and return a default/keep control
/// - return insert_before / insert_after vectors
/// - return replace_with to swap the node for one or more nodes
/// - return remove to drop the node entirely
pub fn visit_document_mut<V: PluginFactory + ?Sized>(
    doc: &mut super::PeisarDocument,
    visitor: &mut V,
) {
    visit_blocks_vec(&mut doc.children, visitor)
}

fn visit_blocks_vec<V: PluginFactory + ?Sized>(children: &mut Vec<Block>, visitor: &mut V) {
    let mut i = 0usize;
    while i < children.len() {
        // take ownership of the node so the visitor can freely replace/remove it
        let mut node = children.remove(i);

        // call visitor
        let ctrl = visitor.visit_block(&mut node);
        // destructure control so we can move fields
        let VisitControl {
            insert_before,
            insert_after,
            replace_with,
            remove,
            recurse,
        } = ctrl;

        // insert_before: put these at the original position
        if !insert_before.is_empty() {
            let before_len = insert_before.len();
            children.splice(i..i, insert_before);
            i += before_len;
        }

        // removal requested
        if remove {
            if !insert_after.is_empty() {
                let after_len = insert_after.len();
                children.splice(i..i, insert_after);
                i += after_len;
            }
            // original removed, continue (don't increment i)
            continue;
        }

        // replacement requested
        if let Some(reps) = replace_with {
            let replacement_len = reps.len();
            children.splice(i..i, reps);
            i += replacement_len;
            continue;
        }

        // keep the (possibly mutated) node; recurse into children if requested
        if recurse {
            match &mut node {
                Block::Heading {
                    children: inline_children,
                    ..
                }
                | Block::Paragraph {
                    children: inline_children,
                    ..
                } => {
                    visit_inline_vec(inline_children, visitor);
                }
                Block::BlockQuote {
                    children: block_children,
                    ..
                } => {
                    visit_blocks_vec(block_children, visitor);
                }
                Block::List { items, .. } => {
                    for item in items.iter_mut() {
                        visit_blocks_vec(&mut item.children, visitor);
                    }
                }
                Block::Table { table, .. } => {
                    visit_table(table, visitor);
                }
                // CodeBlock, ThematicBreak, HtmlBlock, LinkReferenceDefinition,
                // and Comment have no nested AST children
                _ => {}
            }
        }

        // insert the original (possibly mutated) node back into the vector
        children.insert(i, node);
        i += 1;

        // insert_after
        if !insert_after.is_empty() {
            let after_len = insert_after.len();
            children.splice(i..i, insert_after);
            i += after_len;
        }
    }
}

fn visit_table<V: PluginFactory + ?Sized>(table: &mut crate::ast::tokens::Table, visitor: &mut V) {
    for cell in table.header.cells.iter_mut() {
        visit_inline_vec(&mut cell.children, visitor);
    }
    for row in table.rows.iter_mut() {
        for cell in row.cells.iter_mut() {
            visit_inline_vec(&mut cell.children, visitor);
        }
    }
}

fn visit_inline_vec<V: PluginFactory + ?Sized>(inline: &mut Vec<Inline>, visitor: &mut V) {
    let mut i = 0usize;
    while i < inline.len() {
        let mut node = inline.remove(i);
        let ctrl = visitor.visit_inline(&mut node);
        let InlineVisitControl {
            insert_before,
            insert_after,
            replace_with,
            remove,
            recurse,
        } = ctrl;

        if !insert_before.is_empty() {
            let before_len = insert_before.len();
            inline.splice(i..i, insert_before);
            i += before_len;
        }

        if remove {
            if !insert_after.is_empty() {
                let after_len = insert_after.len();
                inline.splice(i..i, insert_after);
                i += after_len;
            }
            continue;
        }
        #[allow(unused_mut)]
        if let Some(reps) = replace_with {
            inline.splice(i..i, reps);
            continue;
        }

        if recurse {
            match &mut node {
                Inline::Emphasis { children, .. } | Inline::Strikethrough { children, .. } => {
                    visit_inline_vec(children, visitor);
                }
                Inline::Link { text, .. } | Inline::LinkReference { text, .. } => {
                    visit_inline_vec(text, visitor);
                }
                // Text, Code, Image, HtmlInline, HardBreak, SoftBreak have no nested inline
                _ => {}
            }
        }

        inline.insert(i, node);
        i += 1;

        if !insert_after.is_empty() {
            let after_len = insert_after.len();
            inline.splice(i..i, insert_after);
            i += after_len;
        }
    }
}
// ---------------------------------------------------------------------------
// Hook registry
// ---------------------------------------------------------------------------

/// Ordered registry of [`PluginFactory`] hooks, threaded through the block and
/// inline parsers.
///
/// Hooks are tried in registration order; the first hook to claim a
/// position wins.
#[derive(Clone, Default)]
pub struct ParserHooks<'a> {
    parsers: Vec<&'a dyn PluginFactory>,
}

impl<'a> ParserHooks<'a> {
    /// An empty registry — no hooks run (the default when none are
    /// registered).
    pub fn empty() -> Self {
        Self {
            parsers: Vec::new(),
        }
    }

    /// Register a hook (appended after any existing hooks).
    pub fn push(&mut self, parser: &'a dyn PluginFactory) {
        self.parsers.push(parser);
    }

    /// Chainable [`push`](Self::push) — returns the registry so calls can
    /// be chained: `ParserHooks::empty().with(&a).with(&b)`.
    pub fn with(mut self, parser: &'a dyn PluginFactory) -> Self {
        self.parsers.push(parser);
        self
    }

    /// `true` when no hooks are registered.
    pub fn is_empty(&self) -> bool {
        self.parsers.is_empty()
    }

    /// Try every registered block hook in order.
    pub fn try_parse_block(&self, ctx: &BlockParserContext) -> Option<(Block, usize)> {
        for parser in &self.parsers {
            if let Some(result) = parser.try_parse_block(ctx) {
                return Some(result);
            }
        }
        None
    }

    /// Try every registered inline hook in order.
    pub fn try_parse_inline(&self, ctx: &InlineParserContext) -> Option<(Inline, usize)> {
        for parser in &self.parsers {
            if let Some(result) = parser.try_parse_inline(ctx) {
                return Some(result);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Node finalization helpers
// ---------------------------------------------------------------------------

/// Overwrite a hook-returned block's source span (computed from the number
/// of consumed lines) and apply a Kramdown `{:...}` attribute block if one
/// follows the block in the source.
pub(crate) fn finalize_hook_block(
    block: Block,
    pos: Span,
    trailing_attrs: Option<Attributes>,
) -> Block {
    match block {
        Block::Heading {
            level,
            children,
            attrs,
            ..
        } => Block::Heading {
            level,
            children,
            pos,
            attrs: trailing_attrs.or(attrs),
        },
        Block::Paragraph {
            children, attrs, ..
        } => Block::Paragraph {
            children,
            pos,
            attrs: trailing_attrs.or(attrs),
        },
        Block::CodeBlock {
            lang, code, attrs, ..
        } => Block::CodeBlock {
            lang,
            code,
            pos,
            attrs: trailing_attrs.or(attrs),
        },
        Block::BlockQuote {
            children, attrs, ..
        } => Block::BlockQuote {
            children,
            pos,
            attrs: trailing_attrs.or(attrs),
        },
        Block::List {
            ordered,
            start,
            tight,
            items,
            attrs,
            ..
        } => Block::List {
            ordered,
            start,
            tight,
            items,
            pos,
            attrs: trailing_attrs.or(attrs),
        },
        Block::ThematicBreak { .. } => Block::ThematicBreak { pos },
        Block::HtmlBlock { html, attrs, .. } => Block::HtmlBlock {
            html,
            pos,
            attrs: trailing_attrs.or(attrs),
        },
        Block::Table { table, attrs, .. } => Block::Table {
            table,
            pos,
            attrs: trailing_attrs.or(attrs),
        },
        Block::LinkReferenceDefinition {
            label, url, title, ..
        } => Block::LinkReferenceDefinition {
            label,
            url,
            title,
            pos,
        },
        Block::Comment { value, .. } => Block::Comment { value, pos },
    }
}

/// Overwrite a hook-returned inline node's source span (computed from the
/// number of consumed characters).
pub(crate) fn finalize_hook_inline(node: Inline, pos: Span) -> Inline {
    match node {
        Inline::Text { value, .. } => Inline::Text { value, pos },
        Inline::Emphasis {
            level, children, ..
        } => Inline::Emphasis {
            level,
            children,
            pos,
        },
        Inline::Code { code, .. } => Inline::Code { code, pos },
        Inline::HtmlInline { html, .. } => Inline::HtmlInline { html, pos },
        Inline::Strikethrough { children, .. } => Inline::Strikethrough { children, pos },
        Inline::HardBreak { .. } => Inline::HardBreak { pos },
        Inline::SoftBreak { .. } => Inline::SoftBreak { pos },
        Inline::Image {
            alt, url, title, ..
        } => Inline::Image {
            alt,
            url,
            title,
            pos,
        },
        Inline::Link {
            text,
            url,
            title,
            autolink,
            ..
        } => Inline::Link {
            text,
            url,
            title,
            autolink,
            pos,
        },
        Inline::LinkReference {
            text,
            label,
            url,
            title,
            ..
        } => Inline::LinkReference {
            text,
            label,
            url,
            title,
            pos,
        },
    }
}
