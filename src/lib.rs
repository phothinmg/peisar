//! Markdown parsing and rendering with an extensible abstract syntax tree.
//!
//! [`Peisar`] parses a Markdown document and exposes its AST, rendered HTML,
//! and front matter. Configure parsing and rendering with [`PeisarOptions`].
//! The [`ast`] and [`plugin`] modules re-export the AST types and plugin APIs.

pub mod ast;
mod core;
pub mod plugin;

pub use core::{
    GetOptions, Peisar, PeisarOptions, RenderOptions, get_options, render_document_html,
};
