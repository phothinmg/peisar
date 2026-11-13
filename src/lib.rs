//! Markdown parsing and rendering with an extensible abstract syntax tree.
//!
//! [`Peisar`] parses a Markdown document and exposes its AST, rendered HTML,
//! and front matter. Configure parsing and rendering with [`PeisarOptions`].
//! The [`ast`] and [`plugin`] modules re-export the AST types and plugin APIs.

pub mod ast;
mod config;
mod html;
mod parser;

pub use config::PeisarOptions;
pub use parser::Peisar;
