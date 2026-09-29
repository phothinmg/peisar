use crate::markdown::ast::AstOptions;
use crate::markdown::html::RenderOptions;
use napi_derive::napi;
use serde::{Deserialize, Serialize};

/// JavaScript options for Markdown parsing and HTML rendering.
///
/// Every property is optional. Omitted parsing options enable GFM and
/// Kramdown; omitted rendering options produce a complete HTML document with
/// charset and viewport metadata.
#[napi(object)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeisarOptions {
    /// Enable GitHub Flavored Markdown (tables, strikethrough, task lists,
    /// autolinks).  Default: `true`.
    pub gfm: Option<bool>,
    /// Enable Kramdown-style block attributes (`{:#id .class key="val"}`).
    /// Default: `true`.
    pub kramdown: Option<bool>,
    /// Optional file name to attach to the parsed [`Document`](crate::Document).
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

pub struct GetOptions {
    pub ast_opts: AstOptions,
    pub render_opts: RenderOptions,
}

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
}
