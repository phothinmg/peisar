//! Combined JavaScript-facing options for Markdown parsing and HTML rendering.
//!
//! [`PeisarOptions`] is the single options object JS consumers pass to the
//! `Peisar` constructor.  It is split internally by [`get_options`] into
//! [`AstOptions`] (parsing) and [`RenderOptions`] (rendering).

use crate::markdown::ast::AstOptions;
use crate::markdown::html::RenderOptions;
use napi_derive::napi;
use serde::{Deserialize, Serialize};

/// JavaScript options for Markdown parsing and HTML rendering.
///
/// Every property is optional.  Omitted parsing options enable GFM and
/// Kramdown; omitted rendering options produce a complete HTML document with
/// charset and viewport metadata.
///
/// # Defaults
///
/// | Property   | Default     |
/// |------------|-------------|
/// | `gfm`      | `true`      |
/// | `kramdown` | `true`      |
/// | `fragment` | `false` (full document) |
/// | `charset`  | `true`      |
/// | `viewport` | `true`      |
/// | `file_name`, `title`, `body_class`, `style` | `null` |
#[napi(object)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeisarOptions {
    /// Enable GitHub Flavored Markdown (tables, strikethrough, task lists,
    /// autolinks).  Default: `true`.
    pub gfm: Option<bool>,
    /// Enable Kramdown-style block attributes (`{:#id .class key="val"}`).
    /// Default: `true`.
    pub kramdown: Option<bool>,
    /// Optional file name to attach to the parsed
    /// [`Document`](crate::markdown::ast::Document).
    pub file_name: Option<String>,
    /// If `true`, emit only the body content (no `<!DOCTYPE>`, `<html>`,
    /// `<head>`, or `<body>` wrapper).  If `false`, emit a full HTML
    /// document.  Default: `true` (fragment).
    pub fragment: Option<bool>,
    /// Include a `<meta charset="utf-8">` in the head (only relevant when
    /// `fragment` is `false`).  Default: `true`.
    pub charset: Option<bool>,
    /// Include a `<meta name="viewport" content="width=device-width,
    /// initial-scale=1.0">` in the head (only relevant when `fragment` is
    /// `false`).  Default: `true`.
    pub viewport: Option<bool>,
    /// Optional `<title>` for the HTML head (only relevant when
    /// `fragment` is `false`).  Default: `None`.
    pub title: Option<String>,
    /// Optional additional CSS classes to add to `<body>` (only relevant
    /// when `fragment` is `false`).  Default: `None`.
    pub body_class: Option<String>,
    /// Optional inline CSS to inject in a `<style>` tag in the head.
    /// Default: `None`.
    pub style: Option<String>,
}
#[napi]
impl Default for PeisarOptions {
    fn default() -> Self {
        Self {
            gfm: Some(true),
            kramdown: Some(true),
            file_name: None,
            fragment: Some(false),
            charset: Some(true),
            viewport: Some(true),
            title: None,
            body_class: None,
            style: None,
        }
    }
}

/// The resolved pair of option structs produced by [`get_options`].
///
/// `ast_opts` drive the parser; `render_opts` drive the HTML renderer.
pub struct GetOptions {
    /// Resolved parsing options.
    pub ast_opts: AstOptions,
    /// Resolved rendering options.
    pub render_opts: RenderOptions,
}

/// Split a [`PeisarOptions`] (or `None`) into resolved [`AstOptions`] and
/// [`RenderOptions`], applying documented defaults to every omitted field.
///
/// # Example
///
/// ```
/// use peisar::markdown::config::{PeisarOptions, get_options};
///
/// // Passing `None` uses the documented defaults.
/// let resolved = get_options(None);
/// assert!(resolved.ast_opts.gfm);
/// assert!(resolved.ast_opts.kramdown);
/// assert!(!resolved.render_opts.fragment); // full document
/// ```
pub fn get_options(options: Option<PeisarOptions>) -> GetOptions {
    let opts = options.unwrap_or_default();
    let ast_opts = AstOptions {
        gfm: opts.gfm.unwrap_or(true),
        kramdown: opts.kramdown.unwrap_or(true),
        file_name: opts.file_name,
    };
    let render_opts = RenderOptions {
        fragment: opts.fragment.unwrap_or(false),
        charset: opts.charset.unwrap_or(true),
        viewport: opts.viewport.unwrap_or(true),
        title: opts.title,
        body_class: opts.body_class,
        style: opts.style,
    };
    GetOptions {
        ast_opts,
        render_opts,
    }
}

#[cfg(test)]
mod tests {
    use super::{PeisarOptions, get_options};

    #[test]
    fn partial_options_use_defaults_for_omitted_fields() {
        let options = PeisarOptions {
            gfm: None,
            kramdown: None,
            file_name: None,
            fragment: Some(true),
            charset: None,
            viewport: None,
            title: None,
            body_class: None,
            style: None,
        };

        let resolved = get_options(Some(options));

        assert!(resolved.ast_opts.gfm);
        assert!(resolved.ast_opts.kramdown);
        assert!(resolved.render_opts.fragment);
        assert!(resolved.render_opts.charset);
        assert!(resolved.render_opts.viewport);
    }

    #[test]
    fn none_resolves_to_documented_defaults() {
        let resolved = get_options(None);

        // Parsing defaults: GFM + Kramdown, no file name.
        assert!(resolved.ast_opts.gfm);
        assert!(resolved.ast_opts.kramdown);
        assert_eq!(resolved.ast_opts.file_name, None);
        // Rendering defaults: full document with charset + viewport.
        assert!(!resolved.render_opts.fragment);
        assert!(resolved.render_opts.charset);
        assert!(resolved.render_opts.viewport);
        assert_eq!(resolved.render_opts.title, None);
        assert_eq!(resolved.render_opts.body_class, None);
        assert_eq!(resolved.render_opts.style, None);
    }

    #[test]
    fn explicit_values_are_preserved() {
        let resolved = get_options(Some(PeisarOptions {
            gfm: Some(false),
            kramdown: Some(false),
            file_name: Some("doc.md".into()),
            fragment: Some(false),
            charset: Some(false),
            viewport: Some(false),
            title: Some("T".into()),
            body_class: Some("body".into()),
            style: Some("s".into()),
        }));

        assert!(!resolved.ast_opts.gfm);
        assert!(!resolved.ast_opts.kramdown);
        assert_eq!(resolved.ast_opts.file_name.as_deref(), Some("doc.md"));
        assert!(!resolved.render_opts.fragment);
        assert!(!resolved.render_opts.charset);
        assert!(!resolved.render_opts.viewport);
        assert_eq!(resolved.render_opts.title.as_deref(), Some("T"));
        assert_eq!(resolved.render_opts.body_class.as_deref(), Some("body"));
        assert_eq!(resolved.render_opts.style.as_deref(), Some("s"));
    }
}
