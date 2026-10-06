//! Custom parser hooks.
//!
//! The [`AstParser`] trait lets extensions parse custom syntax *during*
//! parsing rather than transforming the finished AST afterwards (see the
//! [`AstVisitor`](super::visitor::AstVisitor) trait for post-parse
//! transforms).
//!
//! Two hook kinds exist, mirroring the parser's two phases:
//!
//! - **Block hooks** ([`AstParser::try_parse_block`]) run at the start of
//!   every block, *before* the built-in block matchers.  They receive a
//!   [`BlockParseContext`] and return a parsed [`Block`] plus the number of
//!   source lines consumed, or `None` to decline.
//! - **Inline hooks** ([`AstParser::try_parse_inline`]) run at every
//!   character position of inline content, *before* the built-in inline
//!   matchers.  They receive an [`InlineParseContext`] and return a parsed
//!   [`Inline`] plus the number of characters consumed, or `None` to
//!   decline.
//!
//! Hooks always run before the built-in parsers, so an extension can both
//! introduce new syntax and override built-in constructs.  When several
//! hooks are registered they are tried in registration order and the first
//! one to claim the syntax wins.
//!
//! The returned node's `pos` span is computed automatically by the engine
//! from the hook's `consumed` value — hooks may pass a placeholder span.
//!
use crate::markdown::ast::tokens::{
    Attributes,
    span::Span,
    token::{Block, Inline},
};
#[cfg(feature = "npm")]
use napi_derive::napi;

// ---------------------------------------------------------------------------
// Hook contexts
// ---------------------------------------------------------------------------

/// Context passed to block parser hooks.
///
/// On the JavaScript side the fields are exposed as `line`, `lineIndex`,
/// and `lines`.
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Clone)]
pub struct BlockParseContext {
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
pub struct InlineParseContext {
    /// Remaining inline text starting at the current character position.
    pub rest: String,
    /// 0-based character index of the current position in the inline text.
    pub index: u32,
}

// ---------------------------------------------------------------------------
// Hook trait
// ---------------------------------------------------------------------------

/// Custom parser hook trait.  Implement this to parse extension syntax
/// during parsing.
///
/// Both methods are optional — implement only the phase you need.  Return
/// `None` to decline and let the next hook (or the built-in parser) handle
/// the position.
pub trait AstParser {
    /// Try to parse a block at the current line.
    ///
    /// Returns the parsed [`Block`] and the number of source lines it
    /// consumed (must be at least 1; `0` is treated as `1`), or `None` to
    /// decline.  The returned block's `pos` span is overwritten with an
    /// accurate source span, and a Kramdown `{:...}` attribute block on a
    /// following line is applied automatically.
    fn try_parse_block(&self, _ctx: &BlockParseContext) -> Option<(Block, usize)> {
        None
    }

    /// Try to parse inline content starting at the current character.
    ///
    /// Returns the parsed [`Inline`] node and the number of characters it
    /// consumed (must be at least 1; `0` makes the result be ignored), or
    /// `None` to decline.  The returned node's `pos` span is overwritten
    /// with an accurate source span.
    fn try_parse_inline(&self, _ctx: &InlineParseContext) -> Option<(Inline, usize)> {
        None
    }
}

// ---------------------------------------------------------------------------
// Hook registry
// ---------------------------------------------------------------------------

/// Ordered registry of [`AstParser`] hooks, threaded through the block and
/// inline parsers.
///
/// Hooks are tried in registration order; the first hook to claim a
/// position wins.
#[derive(Clone, Default)]
pub struct ParseHooks<'a> {
    parsers: Vec<&'a dyn AstParser>,
}

impl<'a> ParseHooks<'a> {
    /// An empty registry — no hooks run (the default when none are
    /// registered).
    pub fn empty() -> Self {
        Self {
            parsers: Vec::new(),
        }
    }

    /// Register a hook (appended after any existing hooks).
    pub fn push(&mut self, parser: &'a dyn AstParser) {
        self.parsers.push(parser);
    }

    /// Chainable [`push`](Self::push) — returns the registry so calls can
    /// be chained: `ParseHooks::empty().with(&a).with(&b)`.
    pub fn with(mut self, parser: &'a dyn AstParser) -> Self {
        self.parsers.push(parser);
        self
    }

    /// `true` when no hooks are registered.
    pub fn is_empty(&self) -> bool {
        self.parsers.is_empty()
    }

    /// Try every registered block hook in order.
    pub fn try_parse_block(&self, ctx: &BlockParseContext) -> Option<(Block, usize)> {
        for parser in &self.parsers {
            if let Some(result) = parser.try_parse_block(ctx) {
                return Some(result);
            }
        }
        None
    }

    /// Try every registered inline hook in order.
    pub fn try_parse_inline(&self, ctx: &InlineParseContext) -> Option<(Inline, usize)> {
        for parser in &self.parsers {
            if let Some(result) = parser.try_parse_inline(ctx) {
                return Some(result);
            }
        }
        None
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
