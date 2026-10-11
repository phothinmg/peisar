use crate::markdown::ast::tokens::{
    Attributes,
    span::Span,
    token::{Block, Inline},
};
#[cfg(feature = "npm")]
use napi_derive::napi;

/// Unified plugin factory for the AST engine.
///
/// One trait covers both extension phases — parsing hooks (`try_parse_block` /
/// `try_parse_inline`) and visitor callbacks (`visit_block` / `visit_inline`).
/// It replaces the pre-1.3.0 [`AstParser`] + [`AstVisitor`] pair (both are
/// still available, deprecated).
///
/// All methods have default no-op / declining implementations — implement
/// only the callbacks a plugin needs.
pub trait PluginFactory {
    fn try_parse_block(&self, _ctx: &BlockParserContext) -> Option<(Block, usize)> {
        None
    }
    fn try_parse_inline(&self, _ctx: &InlineParserContext) -> Option<(Inline, usize)> {
        None
    }
    fn visit_block(&mut self, _block: &mut Block) -> VisitControl {
        VisitControl::default()
    }
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

/// Visitor trait for AST nodes. Implement this trait to receive callbacks
/// for every block- and inline-level node. The visitor methods receive a
/// mutable reference and return a control value that can request the
/// node be replaced, removed, or have nodes inserted around it.
///
/// Deprecated since 1.3.0 — implement [`PluginFactory`] instead; its
/// `visit_block` / `visit_inline` methods have the same shape.
#[deprecated(since = "1.3.0", note = "Use `PluginFactory` instead")]
pub trait AstVisitor {
    /// Called for every block-level node (pre-order). Return a
    /// `VisitControl` to indicate modifications. The callback may also
    /// mutate the node in-place.
    fn visit_block(&mut self, _block: &mut Block) -> VisitControl {
        VisitControl::default()
    }

    /// Called for every inline-level node (pre-order). Return an
    /// `InlineVisitControl` to indicate modifications. The callback may
    /// also mutate the inline node in-place.
    fn visit_inline(&mut self, _inline: &mut Inline) -> InlineVisitControl {
        InlineVisitControl::default()
    }
}

/// Custom parser hook trait.  Implement this to parse extension syntax
/// during parsing.
///
/// Both methods are optional — implement only the phase you need.  Return
/// `None` to decline and let the next hook (or the built-in parser) handle
/// the position.
///
/// Deprecated since 1.3.0 — implement [`PluginFactory`] instead; its
/// `try_parse_block` / `try_parse_inline` methods have the same shape.
#[deprecated(since = "1.3.0", note = "Use `PluginFactory` instead")]
pub trait AstParser {
    /// Try to parse a block at the current line.
    ///
    /// Returns the parsed [`Block`] and the number of source lines it
    /// consumed (must be at least 1; `0` is treated as `1`), or `None` to
    /// decline.  The returned block's `pos` span is overwritten with an
    /// accurate source span, and a Kramdown `{:...}` attribute block on a
    /// following line is applied automatically.
    fn try_parse_block(&self, _ctx: &BlockParserContext) -> Option<(Block, usize)> {
        None
    }

    /// Try to parse inline content starting at the current character.
    ///
    /// Returns the parsed [`Inline`] node and the number of characters it
    /// consumed (must be at least 1; `0` makes the result be ignored), or
    /// `None` to decline.  The returned node's `pos` span is overwritten
    /// with an accurate source span.
    fn try_parse_inline(&self, _ctx: &InlineParserContext) -> Option<(Inline, usize)> {
        None
    }
}

#[cfg_attr(feature = "npm", napi(object))]
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
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Clone)]
pub struct InlineParserContext {
    /// Remaining inline text starting at the current character position.
    pub rest: String,
    /// 0-based character index of the current position in the inline text.
    pub index: u32,
}

/// Deprecated pre-1.3.0 name for [`BlockParserContext`].
#[deprecated(since = "1.3.0", note = "Use `BlockParserContext` instead")]
pub type BlockParseContext = BlockParserContext;

/// Deprecated pre-1.3.0 name for [`InlineParserContext`].
#[deprecated(since = "1.3.0", note = "Use `InlineParserContext` instead")]
pub type InlineParseContext = InlineParserContext;
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
pub fn visit_document_mut<V: PluginFactory + ?Sized>(doc: &mut super::Document, visitor: &mut V) {
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
                    visit_inlines_vec(inline_children, visitor);
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

fn visit_table<V: PluginFactory + ?Sized>(
    table: &mut crate::markdown::ast::tokens::token::Table,
    visitor: &mut V,
) {
    for cell in table.header.cells.iter_mut() {
        visit_inlines_vec(&mut cell.children, visitor);
    }
    for row in table.rows.iter_mut() {
        for cell in row.cells.iter_mut() {
            visit_inlines_vec(&mut cell.children, visitor);
        }
    }
}

fn visit_inlines_vec<V: PluginFactory + ?Sized>(inlines: &mut Vec<Inline>, visitor: &mut V) {
    let mut i = 0usize;
    while i < inlines.len() {
        let mut node = inlines.remove(i);
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
            inlines.splice(i..i, insert_before);
            i += before_len;
        }

        if remove {
            if !insert_after.is_empty() {
                let after_len = insert_after.len();
                inlines.splice(i..i, insert_after);
                i += after_len;
            }
            continue;
        }
        #[allow(unused_mut)]
        if let Some(reps) = replace_with {
            inlines.splice(i..i, reps);
            continue;
        }

        if recurse {
            match &mut node {
                Inline::Emphasis { children, .. } | Inline::Strikethrough { children, .. } => {
                    visit_inlines_vec(children, visitor);
                }
                Inline::Link { text, .. } | Inline::LinkReference { text, .. } => {
                    visit_inlines_vec(text, visitor);
                }
                // Text, Code, Image, HtmlInline, HardBreak, SoftBreak have no nested inlines
                _ => {}
            }
        }

        inlines.insert(i, node);
        i += 1;

        if !insert_after.is_empty() {
            let after_len = insert_after.len();
            inlines.splice(i..i, insert_after);
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
    /// be chained: `ParseHooks::empty().with(&a).with(&b)`.
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

/// Deprecated pre-1.3.0 name for [`ParserHooks`].
#[deprecated(since = "1.3.0", note = "Use `ParserHooks` instead")]
pub type ParseHooks<'a> = ParserHooks<'a>;

// ---------------------------------------------------------------------------
// Back-compat: run pre-1.3.0 `AstParser` / `AstVisitor` implementations
// through the unified `PluginFactory` engine without duplicating any
// traversal or registry logic.  A blanket impl for every deprecated-trait
// implementor would conflict with the new one, so the shims below wrap a
// single hook and expose it as a `PluginFactory`.
// ---------------------------------------------------------------------------

/// Adapter that exposes a deprecated [`AstParser`] as a [`PluginFactory`]
/// (parse hooks only).
///
/// ```ignore
/// let adapter = AstParserAdapter::new(&my_old_hook);
/// let hooks = ParserHooks::empty().with(&adapter);
/// let doc = md_to_ast_with_hooks(md, &opts, None, &hooks);
/// ```
#[allow(deprecated)]
#[deprecated(since = "1.3.0", note = "Use `PluginFactory` instead")]
pub struct AstParserAdapter<'a> {
    pub(crate) inner: &'a dyn AstParser,
}

#[allow(deprecated)]
impl<'a> AstParserAdapter<'a> {
    /// Wrap a deprecated [`AstParser`] hook.
    #[deprecated(since = "1.3.0", note = "implement `PluginFactory` instead")]
    pub fn new(parser: &'a dyn AstParser) -> Self {
        Self { inner: parser }
    }
}

#[allow(deprecated)]
impl PluginFactory for AstParserAdapter<'_> {
    fn try_parse_block(&self, ctx: &BlockParserContext) -> Option<(Block, usize)> {
        self.inner.try_parse_block(ctx)
    }
    fn try_parse_inline(&self, ctx: &InlineParserContext) -> Option<(Inline, usize)> {
        self.inner.try_parse_inline(ctx)
    }
}

/// Adapter that exposes a deprecated [`AstVisitor`] as a [`PluginFactory`]
/// (visit callbacks only).
///
/// ```ignore
/// let mut adapter = AstVisitorAdapter::new(&mut my_old_visitor);
/// visit_document_mut(&mut doc, &mut adapter);
/// ```
#[allow(deprecated)]
#[deprecated(since = "1.3.0", note = "Use `PluginFactory` instead")]
pub struct AstVisitorAdapter<'a> {
    pub(crate) inner: &'a mut dyn AstVisitor,
}

#[allow(deprecated)]
impl<'a> AstVisitorAdapter<'a> {
    /// Wrap a deprecated [`AstVisitor`].
    #[deprecated(since = "1.3.0", note = "implement `PluginFactory` instead")]
    pub fn new(visitor: &'a mut dyn AstVisitor) -> Self {
        Self { inner: visitor }
    }
}

#[allow(deprecated)]
impl PluginFactory for AstVisitorAdapter<'_> {
    fn visit_block(&mut self, block: &mut Block) -> VisitControl {
        self.inner.visit_block(block)
    }
    fn visit_inline(&mut self, inline: &mut Inline) -> InlineVisitControl {
        self.inner.visit_inline(inline)
    }
}

// ---------------------------------------------------------------------------
// Node finalisation helpers
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
            items,
            attrs,
            ..
        } => Block::List {
            ordered,
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

// ------------------------------------------------------------------------------------
#[allow(deprecated)]
#[deprecated(since = "1.3.0", note = "Use `visit_document_mut` instead")]
pub fn visit_document_mut_dep<V: AstVisitor + ?Sized>(doc: &mut super::Document, visitor: &mut V) {
    visit_blocks_vec_dep(&mut doc.children, visitor)
}
#[allow(deprecated)]
fn visit_blocks_vec_dep<V: AstVisitor + ?Sized>(children: &mut Vec<Block>, visitor: &mut V) {
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
                    visit_inlines_vec_dep(inline_children, visitor);
                }
                Block::BlockQuote {
                    children: block_children,
                    ..
                } => {
                    visit_blocks_vec_dep(block_children, visitor);
                }
                Block::List { items, .. } => {
                    for item in items.iter_mut() {
                        visit_blocks_vec_dep(&mut item.children, visitor);
                    }
                }
                Block::Table { table, .. } => {
                    visit_table_dep(table, visitor);
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
#[allow(deprecated)]
fn visit_table_dep<V: AstVisitor + ?Sized>(
    table: &mut crate::markdown::ast::tokens::token::Table,
    visitor: &mut V,
) {
    for cell in table.header.cells.iter_mut() {
        visit_inlines_vec_dep(&mut cell.children, visitor);
    }
    for row in table.rows.iter_mut() {
        for cell in row.cells.iter_mut() {
            visit_inlines_vec_dep(&mut cell.children, visitor);
        }
    }
}
#[allow(deprecated)]
fn visit_inlines_vec_dep<V: AstVisitor + ?Sized>(inlines: &mut Vec<Inline>, visitor: &mut V) {
    let mut i = 0usize;
    while i < inlines.len() {
        let mut node = inlines.remove(i);
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
            inlines.splice(i..i, insert_before);
            i += before_len;
        }

        if remove {
            if !insert_after.is_empty() {
                let after_len = insert_after.len();
                inlines.splice(i..i, insert_after);
                i += after_len;
            }
            continue;
        }
        #[allow(unused_mut)]
        if let Some(reps) = replace_with {
            inlines.splice(i..i, reps);
            continue;
        }

        if recurse {
            match &mut node {
                Inline::Emphasis { children, .. } | Inline::Strikethrough { children, .. } => {
                    visit_inlines_vec_dep(children, visitor);
                }
                Inline::Link { text, .. } | Inline::LinkReference { text, .. } => {
                    visit_inlines_vec_dep(text, visitor);
                }
                // Text, Code, Image, HtmlInline, HardBreak, SoftBreak have no nested inlines
                _ => {}
            }
        }

        inlines.insert(i, node);
        i += 1;

        if !insert_after.is_empty() {
            let after_len = insert_after.len();
            inlines.splice(i..i, insert_after);
            i += after_len;
        }
    }
}
