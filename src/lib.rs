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
//! - [`yaml`] — YAML parsing helpers for standalone config files and dynamic
//!   values, backed by a maintained YAML 1.2 parser.
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
//! Parse YAML config into a typed Rust struct:
//!
//! ```
//! use peisar::yaml::parse_yaml_config;
//!
//! #[derive(serde::Deserialize)]
//! struct SiteConfig {
//!     title: String,
//!     port: u16,
//! }
//!
//! let config: SiteConfig = parse_yaml_config("title: Hello\nport: 3000\n").unwrap();
//! assert_eq!(config.title, "Hello");
//! assert_eq!(config.port, 3000);
//! ```
//!
//!
//! ## JavaScript surface
//!
//! The crate also compiles as a native addon. JS consumers construct
//! [`Peisar`](markdown::peisar::Peisar) with Markdown plus options and read
//! its `ast`, `html`, `frontmatter`, or `astJson` properties; see the
//! `peisar` npm package for the binding surface.

pub mod cache;
pub mod frontmatter;
pub mod markdown;
pub mod yaml;
