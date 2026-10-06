//! Callback-based wrapper layer for JavaScript (napi-rs) interop.
//!
//! The Rust [`AstVisitor`](crate::AstVisitor) trait cannot be exported
//! directly through napi-rs (traits and generics are not supported).
//! This module provides a parallel, callback-based API that JS consumers
//! use instead:
//!
//! 1. [`VisitControlJs`] / [`InlineVisitControlJs`] — napi-exported
//!    mirror structs that JS returns from its callback functions.
//! 2. [`BlockCallback`] / [`InlineCallback`] — type-erased napi callbacks
//!    (`ThreadsafeFunction` wrappers).
//! 3. [`JsVisitor`] — napi-exported object shape for the JS callbacks;
//!    the internal adapter forwards them to [`AstVisitor`].
//! 4. [`PeisarAstJs`] — napi-exported high-level struct mirroring
//!    [`PeisarAst`](crate::PeisarAst) but accepting JS callbacks instead of
//!    Rust visitor types.
//!
//! ## JS usage example
//!
//! ```js
//! const { PeisarAstJs } = require("@peisar/ast");
//!
//! const ast = new PeisarAstJs("# Hello\n\nA paragraph.", undefined);
//!
//! // addVisitor takes a single visitor object with two optional
//! // callback functions (a "plugin"):
//! ast.addVisitor({
//!   visitBlock(block) {
//!     if (block.type === "heading") console.log("heading:", block.level);
//!     return { recurse: true };
//!   },
//!   visitInline(inline) {
//!     return {}; // keep, no recurse
//!   },
//! });
//!
//! ast.visitAll();
//! console.log(ast.ast);         // real JS object tree
//! console.log(ast.frontmatter); // real JS object (or null)
//! ```

use crate::markdown::ast::parsers::hooks::{AstParser, BlockParseContext, InlineParseContext};
use crate::markdown::ast::parsers::visitor::{AstVisitor, InlineVisitControl, VisitControl};
use crate::markdown::ast::tokens::token::{Block, Inline};

#[cfg(feature = "npm")]
use napi::bindgen_prelude::{Env, Function, FunctionRef};
#[cfg(feature = "npm")]
use napi_derive::napi;

// ---------------------------------------------------------------------------
// napi-exported control mirrors
// ---------------------------------------------------------------------------

/// JS-facing mirror of [`VisitControl`].
///
/// Returned from the JS `visitBlock` callback.  All fields are optional;
/// omitting a field means "no change" for that operation.
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Default, Clone)]
pub struct VisitorControl {
    /// Nodes to insert before the current node.
    pub insert_before: Option<Vec<Block>>,
    /// Nodes to insert after the current node.
    pub insert_after: Option<Vec<Block>>,
    /// Replace the current node with these nodes.
    pub replace_with: Option<Vec<Block>>,
    /// Remove the current node entirely.
    pub remove: Option<bool>,
    /// Whether to recurse into this node's children.
    pub recurse: Option<bool>,
}

impl From<VisitorControl> for VisitControl {
    fn from(js: VisitorControl) -> Self {
        VisitControl {
            insert_before: js.insert_before.unwrap_or_default(),
            insert_after: js.insert_after.unwrap_or_default(),
            replace_with: js.replace_with,
            remove: js.remove.unwrap_or(false),
            recurse: js.recurse.unwrap_or(false),
        }
    }
}
/// JS-facing mirror of [`InlineVisitControl`].
///
/// Returned from the JS `visitInline` callback.  All fields are optional;
/// omitting a field means "no change" for that operation.
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Default, Clone)]
pub struct InlineVisitorControl {
    /// Nodes to insert before the current node.
    pub insert_before: Option<Vec<Inline>>,
    /// Nodes to insert after the current node.
    pub insert_after: Option<Vec<Inline>>,
    /// Replace the current node with these nodes.
    pub replace_with: Option<Vec<Inline>>,
    /// Remove the current node entirely.
    pub remove: Option<bool>,
    /// Whether to recurse into this node's children.
    pub recurse: Option<bool>,
}

impl From<InlineVisitorControl> for InlineVisitControl {
    fn from(js: InlineVisitorControl) -> Self {
        InlineVisitControl {
            insert_before: js.insert_before.unwrap_or_default(),
            insert_after: js.insert_after.unwrap_or_default(),
            replace_with: js.replace_with,
            remove: js.remove.unwrap_or(false),
            recurse: js.recurse.unwrap_or(false),
        }
    }
}
// ---------------------------------------------------------------------------
// Synchronous JS callbacks (napi FunctionRef wrappers)
// ---------------------------------------------------------------------------
//
// `napi::bindgen_prelude::Function` can only live within the scope of a single
// napi call — it becomes invalid once the call returns.  To store a JS function
// across calls (as a visitor callback), we use `FunctionRef`, which is `Sync`
// and explicitly designed to outlive the original call scope.  When we need to
// invoke it, we call `borrow_back(&env)` to obtain a short-lived `Function`
// bound to the current `Env`, then call it synchronously.
//
// This avoids the deadlock that `ThreadsafeFunction` would cause: visitor
// callbacks run synchronously on the JS main thread (invoked from napi getters
// like `peisar.ast`), so a `ThreadsafeFunction` would queue onto the libuv
// event loop — the same thread we'd be blocking on while waiting for the return
// value.

/// Synchronous block-visitor callback. Receives a `Block`, returns
/// `VisitControlJs` (or `undefined` for no changes).

#[cfg(feature = "npm")]
#[napi]
pub type BlockCallback = FunctionRef<(Block,), Option<VisitorControl>>;
#[cfg(not(feature = "npm"))]
pub type BlockCallback = Box<dyn Fn(Block) -> Option<VisitorControl> + Send + 'static>;
/// Synchronous inline-visitor callback. Receives an `Inline`, returns
/// `InlineVisitControlJs` (or `undefined` for no changes).
#[cfg(feature = "npm")]
#[napi]
pub type InlineCallback = FunctionRef<(Inline,), Option<InlineVisitorControl>>;
#[cfg(not(feature = "npm"))]
pub type InlineCallback = Box<dyn Fn(Inline) -> Option<InlineVisitorControl> + Send + 'static>;
// ---------------------------------------------------------------------------
// JsVisitor object shape and registered AstVisitor adapter
// ---------------------------------------------------------------------------
/// JavaScript object shape for a visitor callback pair.
///
/// On the JS side it is a plain object with two optional
/// function properties:
///
/// ```js
/// const myPlugin = {
///   visitBlock(block)  { return { recurse: true }; },
///   visitInline(inline) { return {}; },
/// };
/// ast.addVisitor(myPlugin);
/// ```
///
/// Either property may be omitted / `null` to skip that node kind.
#[cfg_attr(feature = "npm", napi(object, object_to_js = false))]
#[derive(Default)]
pub struct Visitor {
    /// Optional JS callback for block nodes (JS: `visitBlock`).
    pub visit_block: Option<BlockCallback>,
    /// Optional JS callback for inline nodes (JS: `visitInline`).
    pub visit_inline: Option<InlineCallback>,
}

impl Visitor {
    #[cfg(feature = "npm")]
    pub(crate) fn register(self, env: Env) -> RegisteredVisitorJs {
        RegisteredVisitorJs {
            visit_block: self.visit_block,
            visit_inline: self.visit_inline,
            env,
        }
    }
    #[cfg(not(feature = "npm"))]
    pub(crate) fn register(self) -> RegisteredVisitorRs {
        RegisteredVisitorRs {
            visit_block: self.visit_block,
            visit_inline: self.visit_inline,
        }
    }
}

/// Internal adapter that keeps the environment required by callback references.
#[cfg(feature = "npm")]
pub(crate) struct RegisteredVisitorJs {
    pub visit_block: Option<BlockCallback>,
    pub visit_inline: Option<InlineCallback>,
    /// The napi `Env` captured at registration time, used to `borrow_back`
    /// the `FunctionRef`s when calling them.
    pub env: Env,
}
#[cfg(feature = "npm")]
pub type RegisteredVisitor = RegisteredVisitorJs;
#[cfg(feature = "npm")]
impl AstVisitor for RegisteredVisitorJs {
    #[cfg(feature = "npm")]
    fn visit_block(&mut self, block: &mut Block) -> VisitControl {
        let cb = match &self.visit_block {
            Some(cb) => cb,
            _ => return VisitControl::default(),
        };
        // `borrow_back` creates a short-lived `Function` bound to `env`;
        // calling it runs the JS function inline on this (main) thread.
        let func: Function<(Block,), Option<VisitorControl>> = match cb.borrow_back(&self.env) {
            Ok(f) => f,
            Err(_) => return VisitControl::default(),
        };
        let snapshot = block.clone();
        // `undefined` (no result) means "keep, no changes".
        let js_ctrl = func.call((snapshot,)).unwrap_or(None).unwrap_or_default();
        js_ctrl.into()
    }
    #[cfg(feature = "npm")]
    fn visit_inline(&mut self, inline: &mut Inline) -> InlineVisitControl {
        let cb = match &self.visit_inline {
            Some(cb) => cb,
            _ => return InlineVisitControl::default(),
        };
        let func: Function<(Inline,), Option<InlineVisitorControl>> =
            match cb.borrow_back(&self.env) {
                Ok(f) => f,
                Err(_) => return InlineVisitControl::default(),
            };
        let snapshot = inline.clone();
        // `undefined` (no result) means "keep, no changes".
        let js_ctrl = func.call((snapshot,)).unwrap_or(None).unwrap_or_default();
        js_ctrl.into()
    }
}
#[cfg(not(feature = "npm"))]
pub(crate) struct RegisteredVisitorRs {
    pub visit_block: Option<BlockCallback>,
    pub visit_inline: Option<InlineCallback>,
}
#[cfg(not(feature = "npm"))]
pub(crate) type RegisteredVisitor = RegisteredVisitorRs;
#[cfg(not(feature = "npm"))]
impl AstVisitor for RegisteredVisitorRs {
    fn visit_block(&mut self, block: &mut Block) -> VisitControl {
        let cb = match &self.visit_block {
            Some(cb) => cb,
            _ => return VisitControl::default(),
        };
        let snapshot = block.clone();
        cb(snapshot).unwrap_or_default().into()
    }
    fn visit_inline(&mut self, inline: &mut Inline) -> InlineVisitControl {
        let cb = match &self.visit_inline {
            Some(cb) => cb,
            _ => return InlineVisitControl::default(),
        };
        let snapshot = inline.clone();
        cb(snapshot).unwrap_or_default().into()
    }
}
// ---------------------------------------------------------------------------
// Custom parser hooks (JS interop)
// ---------------------------------------------------------------------------
//
// Mirrors the visitor pattern above: the JS-side `Parser` object holds two
// optional sync callbacks (`parseBlock` / `parseInline`), each receiving a
// context object and returning `{ block, consumed }` / `{ inline, consumed }`
// or `undefined` to decline.  `RegisteredJsParser` adapts them to the Rust
// [`AstParser`] trait so they can be threaded through the parsing engine via
// [`ParseHooks`].

/// Result returned from the JS `parseBlock` callback.
///
/// All fields are optional; omitting `block` (or returning `undefined`)
/// declines the position so the next hook / built-in parser handles it.
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Default, Clone)]
pub struct BlockParseResult {
    /// The parsed block node.
    pub block: Option<Block>,
    /// Number of source lines consumed (must be at least `1`).
    pub consumed: Option<u32>,
}

/// Result returned from the JS `parseInline` callback.
///
/// All fields are optional; omitting `inline` (or returning `undefined`)
/// declines the position so the next hook / built-in parser handles it.
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Default, Clone)]
pub struct InlineParseResult {
    /// The parsed inline node.
    pub inline: Option<Inline>,
    /// Number of characters consumed (`0` or missing declines).
    pub consumed: Option<u32>,
}

/// Synchronous block parser hook. Receives a [`BlockParseContext`], returns
/// a [`BlockParseResultJs`] (or `undefined` to decline).
#[cfg(feature = "npm")]
#[napi]
pub type BlockParseCallback = FunctionRef<(BlockParseContext,), Option<BlockParseResult>>;
#[cfg(not(feature = "npm"))]
pub type BlockParseCallback =
    Box<dyn Fn(BlockParseContext) -> Option<BlockParseResult> + Send + 'static>;
/// Synchronous inline parser hook. Receives an [`InlineParseContext`],
/// returns an [`InlineParseResultJs`] (or `undefined` to decline).
#[cfg(feature = "npm")]
#[napi]
pub type InlineParseCallback = FunctionRef<(InlineParseContext,), Option<InlineParseResult>>;
#[cfg(not(feature = "npm"))]
pub type InlineParseCallback =
    Box<dyn Fn(InlineParseContext) -> Option<InlineParseResult> + Send + 'static>;
/// Either property may be omitted / `null` to skip that phase.
#[cfg_attr(feature = "npm", napi(object, object_to_js = false))]
#[derive(Default)]
pub struct Parser {
    /// Optional JS block parser hook (JS: `parseBlock`).
    pub parse_block: Option<BlockParseCallback>,
    /// Optional JS inline parser hook (JS: `parseInline`).
    pub parse_inline: Option<InlineParseCallback>,
}

impl Parser {
    #[cfg(feature = "npm")]
    pub(crate) fn register(self, env: Env) -> RegisteredParserJs {
        RegisteredParserJs {
            parse_block: self.parse_block,
            parse_inline: self.parse_inline,
            env,
        }
    }

    #[cfg(not(feature = "npm"))]
    pub(crate) fn register(self) -> RegisteredParserRs {
        RegisteredParserRs {
            parse_block: self.parse_block,
            parse_inline: self.parse_inline,
        }
    }
}

/// Internal adapter that keeps the environment required by callback
/// references, implementing the Rust [`AstParser`] trait.
#[cfg(feature = "npm")]
pub(crate) struct RegisteredParserJs {
    pub parse_block: Option<BlockParseCallback>,
    pub parse_inline: Option<InlineParseCallback>,
    /// The napi `Env` captured at registration time, used to `borrow_back`
    /// the `FunctionRef`s when calling them.
    pub env: Env,
}
#[cfg(feature = "npm")]
pub(crate) type RegisteredParser = RegisteredParserJs;
#[cfg(feature = "npm")]
impl AstParser for RegisteredParserJs {
    fn try_parse_block(&self, ctx: &BlockParseContext) -> Option<(Block, usize)> {
        let cb = self.parse_block.as_ref()?;
        let func: Function<(BlockParseContext,), Option<BlockParseResult>> =
            match cb.borrow_back(&self.env) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("{}{}", "parseBlock hook could not be invoked: ", e);
                    return None;
                }
            };
        // `undefined` (no result) declines the position.
        let result = match func.call((ctx.clone(),)) {
            Ok(r) => r?,
            Err(e) => {
                eprintln!(
                    "{}",
                    format!(
                        "parseBlock hook returned an invalid result: {e}\n  \
                     note: the returned `block` must be a complete node — every \
                     field (including `pos` with line/column/offset) is required"
                    )
                );
                return None;
            }
        };
        let block = result.block?;
        let consumed = result.consumed.unwrap_or(1).max(1) as usize;
        Some((block, consumed))
    }

    fn try_parse_inline(&self, ctx: &InlineParseContext) -> Option<(Inline, usize)> {
        let cb = self.parse_inline.as_ref()?;
        let func: Function<(InlineParseContext,), Option<InlineParseResult>> =
            match cb.borrow_back(&self.env) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("parseInline hook could not be invoked: {e}");
                    return None;
                }
            };
        // `undefined` (no result) declines the position.
        let result = match func.call((ctx.clone(),)) {
            Ok(r) => r?,
            Err(e) => {
                eprintln!(
                    "parseInline hook returned an invalid result: {e}\n  \
                     note: the returned `inline` must be a complete node — every \
                     field (including `pos` with line/column/offset) is required"
                );
                return None;
            }
        };
        let inline = result.inline?;
        let consumed = result.consumed.unwrap_or(0) as usize;
        (consumed > 0).then_some((inline, consumed))
    }
}
#[cfg(not(feature = "npm"))]
pub(crate) struct RegisteredParserRs {
    pub parse_block: Option<BlockParseCallback>,
    pub parse_inline: Option<InlineParseCallback>,
}
#[cfg(not(feature = "npm"))]
pub(crate) type RegisteredParser = RegisteredParserRs;
#[cfg(not(feature = "npm"))]
impl AstParser for RegisteredParserRs {
    fn try_parse_block(&self, ctx: &BlockParseContext) -> Option<(Block, usize)> {
        let cb = self.parse_block.as_ref()?;
        let result = cb(ctx.clone())?;
        let block = result.block?;
        let consumed = result.consumed.unwrap_or(1).max(1) as usize;
        Some((block, consumed))
    }
    fn try_parse_inline(&self, ctx: &InlineParseContext) -> Option<(Inline, usize)> {
        let cb = self.parse_inline.as_ref()?;
        let result = cb(ctx.clone())?;
        let inline = result.inline?;
        let consumed = result.consumed.unwrap_or(0) as usize;
        (consumed > 0).then_some((inline, consumed))
    }
}
