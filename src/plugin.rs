//! Plugin types for extending Markdown parsing and transforming AST nodes.
//!
//! This module re-exports the plugin interfaces provided by `peisar_ast`.

pub use peisar_ast::plugin::*;
pub use peisar_ast::{Plugin, RegisteredPlugin};
