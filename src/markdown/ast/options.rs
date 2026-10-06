//! Parser configuration options.
//!
//! The [`AstOptions`] struct controls which Markdown extensions are enabled
//! when parsing.  It is passed to [`Document::parse`](crate::markdown::ast::Document::parse)
//! or [`md_to_ast`](crate::markdown::ast::parsers::md_to_ast).

#[cfg(feature = "npm")]
use napi_derive::napi;
use serde::{Deserialize, Serialize};
/// Options that control how Markdown is parsed.
///
/// # Defaults
///
/// Both GFM and Kramdown extensions are enabled by default.  To parse
/// strict CommonMark only:
///

#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstOptions {
    /// Enable GitHub Flavored Markdown (tables, strikethrough, task lists,
    /// autolinks).  Default: `true`.
    pub gfm: bool,
    /// Enable Kramdown-style block attributes (`{:#id .class key="val"}`).
    /// Default: `true`.
    pub kramdown: bool,
    /// Optional file name to attach to the parsed
    /// [`Document`](crate::markdown::ast::Document).
    pub file_name: Option<String>,
}

impl Default for AstOptions {
    fn default() -> Self {
        Self {
            gfm: true,
            kramdown: true,
            file_name: None,
        }
    }
}
