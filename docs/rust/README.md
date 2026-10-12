# Rust usage

Peisar is a Rust crate first. The Node.js package is built on top of the same
parser, renderer, and front-matter support.

This page is the Rust overview. Use it as the entry point for module-level and
API-specific guides.

## At a glance

- Crate: `peisar`
- API style: module-based public Rust surface
- Shared concepts: parser, renderer, front matter, and plugin traits
- Full API details: see [Rust API reference](./apis.md)
- Cross-language docs: see [Node.js overview](../nodejs/README.md)

## Install

```toml
[dependencies]
peisar = "1.0.0"
```

## Current API structure

The public Rust surface is module-based:

| Module | Purpose |
| --- | --- |
| `peisar::frontmatter` | YAML front matter extraction |
| `peisar::yaml` | Standalone YAML parsing helpers |
| `peisar::markdown::ast` | AST nodes, parser options, traversal helpers, plugin traits |
| `peisar::markdown::config` | `PeisarOptions` shared with the Node-facing wrapper |
| `peisar::markdown::html` | AST-to-HTML rendering |
| `peisar::markdown::peisar` | The high-level `Peisar` wrapper type |

Detailed items and examples are in the [Rust API reference](./apis.md).

## Parse and render

```rust
use peisar::markdown::ast::{AstOptions, Document};
use peisar::markdown::html::{RenderOptions, render_document_html};

let document = Document::parse("# Hello **world**\n", &AstOptions::default(), None);

let html = render_document_html(
    &document,
    Some(RenderOptions {
        fragment: true,
        ..RenderOptions::default()
    }),
);

assert_eq!(html, "<h1>Hello <strong>world</strong></h1>\n");
```

## Front matter

```rust
use peisar::frontmatter::frontmatter;

let parsed = frontmatter("---\ntitle: Hello\n---\n\n# Hello".to_string())?;

assert_eq!(parsed.yaml_data().unwrap()["title"], "Hello");
assert_eq!(parsed.pure_markdown_content(), "# Hello");
# Ok::<(), String>(())
```

## High-level wrapper

```rust
use peisar::markdown::config::PeisarOptions;
use peisar::markdown::peisar::Peisar;

let mut document = Peisar::new(
    "# Hello".into(),
    Some(PeisarOptions {
        fragment: Some(true),
        ..Default::default()
    }),
);

assert_eq!(document.html(), "<h1>Hello</h1>\n");
```

## Related docs

- [Rust API reference](./apis.md)
- [Benchmark results](../benchmarks.md)
- [Node.js overview](../nodejs/README.md)
- [Documentation hub](../README.md)
