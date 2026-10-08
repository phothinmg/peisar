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
//! - [`cache`] — the memory-first [`PeisarCache`](cache::PeisarCache): an
//!   LRU in-memory store backed by a binary shard archive
//!   (`.peisar-cache` + `map.json`), Liquid-ready page/asset objects,
//!   configurable Markdown options with JS visitors/parsers, and
//!   hosting-aware baseUrl resolution — exported to JavaScript for the
//!   `peisar-ssg` package.
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
//! use peisar::frontmatter::frontmatter;
//!
//! let parsed = frontmatter("---\ntitle: Hello\n---\n\n# Hello".to_string()).unwrap();
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
//! [`PeisarCache`](cache::PeisarCache) keeps a site's markdown and assets in
//! a size-bounded LRU in memory, flushes cold content into binary shards
//! under `.peisar-cache` (searchable via `mapJson()`), derives
//! Liquid-ready `PageObject` / `AssetsObject` values (`pages()`,
//! `assets()`, `getPage()`, `getAsset()`), resolves hosting `baseUrl`
//! (`baseUrl()`, `resolveUrl()`), supports custom visitors/parsers
//! (`useVisitor` / `useParser`), and can keep itself in sync with the
//! filesystem via `startWatchingJs()`; see the `peisar` npm package for the
//! binding surface.

pub mod cache;
pub mod frontmatter;
pub mod markdown;
