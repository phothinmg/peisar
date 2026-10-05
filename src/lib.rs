//! # peisar
//!
//! `peisar` is a practical Markdown parser written in Rust and exposed to
//! Node.js through napi-rs.  It parses Markdown (with GFM and Kramdown
//! extensions) into a typed AST, renders that AST to HTML, and extracts YAML
//! front matter.
//!
//! ## Layout
//!
//! - [`frontmatter`] — YAML front-matter extraction (`--- … ---` blocks).
//! - [`cache`] — the [`PeisarCache`](cache::PeisarCache) in-memory + on-disk
//!   file cache with file-watching, exported to JavaScript for
//!   the `peisar-ssg` package.
//! - [`markdown`] — the Markdown pipeline:
//!   - [`markdown::ast`] — the AST (`Block` / `Inline` tokens, options,
//!     parsers, visitor, hooks), plus the JS-interop layer.
//!   - [`markdown::config`] — the JavaScript-facing
//!     [`PeisarOptions`](markdown::config::PeisarOptions) that combine AST
//!     and render options.
//!   - [`markdown::html`] — AST → HTML rendering.
//!   - [`markdown::peisar`] — the `Peisar` class exported to JavaScript.
//!
//! ## Quick start (Rust)
//!
//! Parse Markdown into a [`Document`](markdown::ast::Document) and render it
//! to an HTML fragment:
//!
//! ```
//! use peisar::markdown::ast::{AstOptions, Document};
//! use peisar::markdown::html::{RenderOptions, render_document_html};
//!
//! let doc = Document::parse("# Hello **world**\n", &AstOptions::default(), None);
//! let html = render_document_html(
//!     &doc,
//!     Some(RenderOptions { fragment: true, ..RenderOptions::default() }),
//! );
//! assert_eq!(html, "<h1>Hello <strong>world</strong></h1>\n");
//! ```
//!
//! Extract YAML front matter from a document:
//!
//! ```
//! use peisar::frontmatter::parse_markdown_frontmatter;
//!
//! let parsed = parse_markdown_frontmatter("---\ntitle: Hello\n---\n\n# Hello").unwrap();
//! assert_eq!(parsed.pure_markdown_content(), "# Hello");
//! assert_eq!(parsed.yaml_data().unwrap()["title"], "Hello");
//! ```
//!
//!
//! ## JavaScript surface
//!
//! The crate also compiles as a native addon.  JS consumers construct
//! [`Peisar`](markdown::peisar::Peisar) with Markdown plus options and read
//! its `ast`, `html`, `frontmatter`, or `astJson` properties.
//! [`PeisarCache`](cache::PeisarCache) caches the Markdown and asset files of
//! a site directory in memory (and under `.peisar_cache`), exposes
//! `markdownFiles()`, `assetFiles()`, `listFiles()`, `getText()`, and
//! `getBinary()`, and can keep itself in sync with the filesystem via
//! `startWatchingJs()`; see the `peisar` npm package for the binding surface.

pub mod cache;
pub mod frontmatter;
pub mod markdown;
