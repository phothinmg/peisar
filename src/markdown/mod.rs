//! The Markdown pipeline: parsing, AST, and HTML rendering.
//!
//! - [`ast`] — AST node types, options, the block/inline parsers, the
//!   visitor trait, parser hooks, and the JS interop layer.
//! - [`config`] — the JS-facing [`PeisarOptions`](config::PeisarOptions)
//!   that combine parse + render options in one object.
//! - [`html`] — AST → HTML rendering ([`render_document_html`](html::render_document_html)).
//! - [`peisar`] — the `Peisar` class exported to JavaScript.

pub mod ast;
pub mod config;
pub mod html;
pub mod peisar;
