use crate::markdown::ast::parsers::plugin::{
    BlockParserContext, InlineParserContext, InlineVisitControl, PluginFactory, VisitControl,
};

use crate::markdown::ast::tokens::token::{Block, Inline};

#[cfg(feature = "npm")]
use napi::bindgen_prelude::{Env, Function, FunctionRef};
#[cfg(feature = "npm")]
use napi_derive::napi;

// ------------------------------------------------------------------------------------------------------//

// 1. Napi mirrors

// 1.1 Napi-exported control

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
// 1.1 Custom parser hooks (JS interop)
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

// 2. Callback

// 2.1 Synchronous JS callbacks (napi FunctionRef wrappers)
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
#[cfg(feature = "npm")]
#[napi]
/// Synchronous inline-visitor callback. Receives an `Inline`, returns
/// `InlineVisitControlJs` (or `undefined` for no changes).
pub type InlineCallback = FunctionRef<(Inline,), Option<InlineVisitorControl>>;

/// Synchronous block parser hook. Receives a [`BlockParseContext`], returns
/// a [`BlockParseResultJs`] (or `undefined` to decline).
#[cfg(feature = "npm")]
#[napi]
pub type BlockParseCallback = FunctionRef<(BlockParserContext,), Option<BlockParseResult>>;
/// Synchronous inline parser hook. Receives an [`InlineParseContext`],
/// returns an [`InlineParseResultJs`] (or `undefined` to decline).
#[cfg(feature = "npm")]
#[napi]
pub type InlineParseCallback = FunctionRef<(InlineParserContext,), Option<InlineParseResult>>;

// 2.2 Callback for rust side

#[cfg(not(feature = "npm"))]
pub type BlockCallback = Box<dyn Fn(Block) -> Option<VisitorControl> + Send + 'static>;
#[cfg(not(feature = "npm"))]
pub type InlineCallback = Box<dyn Fn(Inline) -> Option<InlineVisitorControl> + Send + 'static>;
#[cfg(not(feature = "npm"))]
pub type BlockParseCallback =
    Box<dyn Fn(BlockParserContext) -> Option<BlockParseResult> + Send + 'static>;
#[cfg(not(feature = "npm"))]
pub type InlineParseCallback =
    Box<dyn Fn(InlineParserContext) -> Option<InlineParseResult> + Send + 'static>;

// 3. Plugins Control

// 3.1 Callback for Registered Plugins
// New API since 1.0.1

#[cfg_attr(feature = "npm", napi(object, object_to_js = false))]
#[derive(Default)]
pub struct Plugin {
    /// Optional JS callback for block nodes (JS: `visitBlock`).
    pub visit_block: Option<BlockCallback>,
    /// Optional JS callback for inline nodes (JS: `visitInline`).
    pub visit_inline: Option<InlineCallback>,
    /// Optional JS block parser hook (JS: `parseBlock`).
    pub parse_block: Option<BlockParseCallback>,
    /// Optional JS inline parser hook (JS: `parseInline`).
    pub parse_inline: Option<InlineParseCallback>,
}

impl Plugin {
    #[cfg(feature = "npm")]
    pub(crate) fn register(self, env: Env) -> RegisteredPlugin {
        RegisteredPlugin {
            visit_block: self.visit_block,
            visit_inline: self.visit_inline,
            parse_block: self.parse_block,
            parse_inline: self.parse_inline,
            env,
        }
    }
    #[cfg(not(feature = "npm"))]
    pub(crate) fn register(self) -> RegisteredPlugin {
        RegisteredPlugin {
            visit_block: self.visit_block,
            visit_inline: self.visit_inline,
            parse_block: self.parse_block,
            parse_inline: self.parse_inline,
        }
    }
}

// 3.2 Register plugins for napi-rs

#[cfg(feature = "npm")]
pub(crate) struct RegisteredPlugin {
    pub visit_block: Option<BlockCallback>,
    pub visit_inline: Option<InlineCallback>,
    pub parse_block: Option<BlockParseCallback>,
    pub parse_inline: Option<InlineParseCallback>,
    /// The napi `Env` captured at registration time, used to `borrow_back`
    /// the `FunctionRef`s when calling them.
    pub env: Env,
}

#[cfg(feature = "npm")]
impl PluginFactory for RegisteredPlugin {
    #[cfg(feature = "npm")]
    fn has_parse_hooks(&self) -> bool {
        self.parse_block.is_some() || self.parse_inline.is_some()
    }
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
    fn try_parse_block(&self, ctx: &BlockParserContext) -> Option<(Block, usize)> {
        let cb = self.parse_block.as_ref()?;
        let func: Function<(BlockParserContext,), Option<BlockParseResult>> =
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

    fn try_parse_inline(&self, ctx: &InlineParserContext) -> Option<(Inline, usize)> {
        let cb = self.parse_inline.as_ref()?;
        let func: Function<(InlineParserContext,), Option<InlineParseResult>> =
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

// 3.3 Register plugins for rust side

#[cfg(not(feature = "npm"))]
pub(crate) struct RegisteredPlugin {
    pub visit_block: Option<BlockCallback>,
    pub visit_inline: Option<InlineCallback>,
    pub parse_block: Option<BlockParseCallback>,
    pub parse_inline: Option<InlineParseCallback>,
}
#[cfg(not(feature = "npm"))]
impl PluginFactory for RegisteredPlugin {
    fn has_parse_hooks(&self) -> bool {
        self.parse_block.is_some() || self.parse_inline.is_some()
    }
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
    fn try_parse_block(&self, ctx: &BlockParserContext) -> Option<(Block, usize)> {
        let cb = self.parse_block.as_ref()?;
        let result = cb(ctx.clone())?;
        let block = result.block?;
        let consumed = result.consumed.unwrap_or(1).max(1) as usize;
        Some((block, consumed))
    }
    fn try_parse_inline(&self, ctx: &InlineParserContext) -> Option<(Inline, usize)> {
        let cb = self.parse_inline.as_ref()?;
        let result = cb(ctx.clone())?;
        let inline = result.inline?;
        let consumed = result.consumed.unwrap_or(0) as usize;
        (consumed > 0).then_some((inline, consumed))
    }
}

// 4. Deprecated API since v1.3.0 (`AstVisitor` and `AstParser`)
//
// The deprecated `Visitor` / `Parser` objects are thin shims: converting one
// produces the equivalent [`Plugin`], so both old and new registrations flow
// through the exact same adapter + traversal code path.
//
// The module-level `allow` below is load-bearing: the napi and Default
// derive macros generate impls that reference these deprecated structs at
// the definition span, where an item-level `#[allow(deprecated)]` cannot
// reach them.
#[allow(deprecated)]
mod deprecated_js {
    #[cfg(feature = "npm")]
    use super::Env;
    use super::{
        BlockCallback, BlockParseCallback, InlineCallback, InlineParseCallback, Plugin,
        RegisteredPlugin,
    };
    #[cfg(feature = "npm")]
    use napi_derive::napi;

    // 4.1 Visitor — pre-1.3.0 shape (visitBlock / visitInline only)

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
    #[deprecated(since = "1.3.0", note = "Use `Plugin` instead")]
    pub struct Visitor {
        /// Optional JS callback for block nodes (JS: `visitBlock`).
        pub visit_block: Option<BlockCallback>,
        /// Optional JS callback for inline nodes (JS: `visitInline`).
        pub visit_inline: Option<InlineCallback>,
    }
    #[allow(deprecated)]
    impl Visitor {
        /// Convert into the equivalent [`Plugin`] (no callbacks lost).
        #[cfg(feature = "npm")]
        pub(crate) fn into_plugin(self, env: Env) -> RegisteredPlugin {
            Plugin {
                visit_block: self.visit_block,
                visit_inline: self.visit_inline,
                parse_block: None,
                parse_inline: None,
            }
            .register(env)
        }

        #[cfg(not(feature = "npm"))]
        pub(crate) fn into_plugin(self) -> RegisteredPlugin {
            Plugin {
                visit_block: self.visit_block,
                visit_inline: self.visit_inline,
                parse_block: None,
                parse_inline: None,
            }
            .register()
        }
    }

    // 4.2 Parser — pre-1.3.0 shape (parseBlock / parseInline only)

    /// Either property may be omitted / `null` to skip that phase.
    #[cfg_attr(feature = "npm", napi(object, object_to_js = false))]
    #[derive(Default)]
    #[deprecated(since = "1.3.0", note = "Use `Plugin` instead")]
    pub struct Parser {
        /// Optional JS block parser hook (JS: `parseBlock`).
        pub parse_block: Option<BlockParseCallback>,
        /// Optional JS inline parser hook (JS: `parseInline`).
        pub parse_inline: Option<InlineParseCallback>,
    }
    #[allow(deprecated)]
    impl Parser {
        /// Convert into the equivalent [`Plugin`] (no callbacks lost).
        #[cfg(feature = "npm")]
        pub(crate) fn into_plugin(self, env: Env) -> RegisteredPlugin {
            Plugin {
                visit_block: None,
                visit_inline: None,
                parse_block: self.parse_block,
                parse_inline: self.parse_inline,
            }
            .register(env)
        }

        #[cfg(not(feature = "npm"))]
        pub(crate) fn into_plugin(self) -> RegisteredPlugin {
            Plugin {
                visit_block: None,
                visit_inline: None,
                parse_block: self.parse_block,
                parse_inline: self.parse_inline,
            }
            .register()
        }
    }
}

#[allow(deprecated)]
pub use deprecated_js::{Parser, Visitor};
