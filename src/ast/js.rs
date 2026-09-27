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

use crate::ast::parsers::visitor::{AstVisitor, InlineVisitControl, VisitControl};
use crate::ast::tokens::token::{Block, Inline};
use napi::bindgen_prelude::{Env, Function, FunctionRef};
use napi_derive::napi;

// ---------------------------------------------------------------------------
// napi-exported control mirrors
// ---------------------------------------------------------------------------

/// JS-facing mirror of [`VisitControl`].
///
/// Returned from the JS `visitBlock` callback.  All fields are optional;
/// omitting a field means "no change" for that operation.
#[napi(object)]
#[derive(Debug, Default, Clone)]
pub struct VisitControlJs {
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

impl From<VisitControlJs> for VisitControl {
    fn from(js: VisitControlJs) -> Self {
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
#[napi(object)]
#[derive(Debug, Default, Clone)]
pub struct InlineVisitControlJs {
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

impl From<InlineVisitControlJs> for InlineVisitControl {
    fn from(js: InlineVisitControlJs) -> Self {
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
/// `VisitControlJs`.
#[napi]
pub type BlockCallback = FunctionRef<(Block,), VisitControlJs>;

/// Synchronous inline-visitor callback. Receives an `Inline`, returns
/// `InlineVisitControlJs`.
#[napi]
pub type InlineCallback = FunctionRef<(Inline,), InlineVisitControlJs>;

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
#[napi(object, object_to_js = false)]
#[derive(Default)]
pub struct JsVisitor {
    /// Optional JS callback for block nodes (JS: `visitBlock`).
    pub visit_block: Option<BlockCallback>,
    /// Optional JS callback for inline nodes (JS: `visitInline`).
    pub visit_inline: Option<InlineCallback>,
}

impl JsVisitor {
    pub(crate) fn register(self, env: Env) -> RegisteredJsVisitor {
        RegisteredJsVisitor {
            visit_block: self.visit_block,
            visit_inline: self.visit_inline,
            env,
        }
    }
}

/// Internal adapter that keeps the environment required by callback references.
pub(crate) struct RegisteredJsVisitor {
    visit_block: Option<BlockCallback>,
    visit_inline: Option<InlineCallback>,
    /// The napi `Env` captured at registration time, used to `borrow_back`
    /// the `FunctionRef`s when calling them.
    env: Env,
}

impl AstVisitor for RegisteredJsVisitor {
    fn visit_block(&mut self, block: &mut Block) -> VisitControl {
        let cb = match &self.visit_block {
            Some(cb) => cb,
            _ => return VisitControl::default(),
        };
        // `borrow_back` creates a short-lived `Function` bound to `env`;
        // calling it runs the JS function inline on this (main) thread.
        let func: Function<(Block,), VisitControlJs> = match cb.borrow_back(&self.env) {
            Ok(f) => f,
            Err(_) => return VisitControl::default(),
        };
        let snapshot = block.clone();
        let js_ctrl = func.call((snapshot,)).unwrap_or_default();
        js_ctrl.into()
    }

    fn visit_inline(&mut self, inline: &mut Inline) -> InlineVisitControl {
        let cb = match &self.visit_inline {
            Some(cb) => cb,
            _ => return InlineVisitControl::default(),
        };
        let func: Function<(Inline,), InlineVisitControlJs> = match cb.borrow_back(&self.env) {
            Ok(f) => f,
            Err(_) => return InlineVisitControl::default(),
        };
        let snapshot = inline.clone();
        let js_ctrl = func.call((snapshot,)).unwrap_or_default();
        js_ctrl.into()
    }
}
