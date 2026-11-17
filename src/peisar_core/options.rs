//! Parser configuration options.
//!
//! The [`AstOptions`] struct controls which Markdown extensions are enabled
//! when parsing. It is passed to [`PeisarAst`](crate::PeisarAst) or the
//! [`md_to_ast`](crate::parsers::md_to_ast) function.

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

#[cfg_attr(feature = "npm", napi(object, namespace = "peisar"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstOptions {
    /// Enable GitHub Flavored Markdown (tables, strikethrough, task lists,
    /// autolinks).  Default: `true`.
    pub gfm: bool,
    /// Enable Kramdown-style block attributes (`{:#id .class key="val"}`).
    /// Default: `true`.
    pub kramdown: bool,
    /// Optional file name to attach to the parsed document.
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
