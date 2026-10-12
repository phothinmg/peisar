# Rust API reference

This document summarizes the current public Rust-facing API layout.

## `peisar::frontmatter`

Primary entry point:

```rust
use peisar::frontmatter::frontmatter;
```

Returns `FrontmatterResult`, which provides:

- `yaml_data() -> Option<&serde_json::Value>`
- `pure_markdown_content() -> &str`
- `into_parts() -> (String, Option<serde_json::Value>)`

Use it when you want front matter separated before parsing Markdown further.

## `peisar::yaml`

Dynamic YAML helpers:

- `parse_yaml_value(&str) -> Result<serde_json::Value, String>`
- `load_yaml_value_file(path) -> Result<serde_json::Value, String>`

Typed config helpers:

- `parse_yaml_config<T>(&str) -> Result<T, String>`
- `load_yaml_config_file<T>(path) -> Result<T, String>`

## `peisar::markdown::ast`

Key exports:

- `AstOptions`
- `Document`
- `PluginFactory`
- `ParserHooks`
- `visit_document_mut`
- `md_to_ast_with_hooks`

Notable behavior:

- `Document::parse(...)` is the normal entry point for parsing Markdown.
- GFM and Kramdown parsing are configured through `AstOptions`.
- `PluginFactory` is the current unified extension trait for parser hooks and
  visitor callbacks.
- Deprecated compatibility traits `AstVisitor` and `AstParser` still exist.

### Example

```rust
use peisar::markdown::ast::{AstOptions, Document};

let document = Document::parse("# Hello\n", &AstOptions::default(), None);
assert_eq!(document.children.len(), 1);
```

## `peisar::markdown::html`

Key exports:

- `RenderOptions`
- `render_document_html(&Document, Option<RenderOptions>) -> String`
- `AstToHtml`

Use `RenderOptions { fragment: true, .. }` when you want just the rendered
body content instead of a full HTML document.

## `peisar::markdown::config`

Key exports:

- `PeisarOptions`
- `get_options`

`PeisarOptions` is the shared high-level option object used by the Node-facing
wrapper and the Rust `markdown::peisar::Peisar` type.

## `peisar::markdown::peisar`

Key type:

- `Peisar`

Notable methods:

- `Peisar::new(markdown, options)`
- `ast()`
- `html()`
- `frontmatter()`
- `ast_json()`

This wrapper is the closest Rust equivalent to the Node.js `Peisar` class.

## Choosing an API

- Use `markdown::ast` + `markdown::html` for lower-level parsing and rendering.
- Use `markdown::peisar::Peisar` for a higher-level single-document wrapper.
- Use `frontmatter` or `yaml` when you only need metadata/config handling.
