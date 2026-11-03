# Rust API reference

The crate re-exports `Peisar` and `PeisarOptions` at the root. The `ast` and
`plugin` modules are public without feature flags.

## `peisar::{Peisar, PeisarOptions}`

`Peisar` is the high-level single-document parser and renderer. It exposes
`ast()`, `html()`, `frontmatter()`, and `ast_json()` methods.
`PeisarOptions` configures GFM, Kramdown attributes, file metadata, and HTML
output.

```rust
use peisar::{Peisar, PeisarOptions};

let mut document = Peisar::new(
    "# Hello".into(),
    Some(PeisarOptions {
        fragment: Some(true),
        ..Default::default()
    }),
);
assert_eq!(document.html(), "<h1>Hello</h1>\n");
```

## `peisar::ast`

The AST types and parsing helpers are re-exported from `peisar_ast`.

Use `Document::parse` to parse Markdown directly. Use
`md_to_ast_with_hooks` when parsing with custom hooks.

```rust
use peisar::ast::{AstOptions, Document};

let document = Document::parse("# Hello\n", &AstOptions::default(), None);
assert_eq!(document.children.len(), 1);
```

## `peisar::plugin`

This module re-exports the plugin interfaces and visitor helpers from
`peisar_ast`. It is available without enabling a Cargo feature.

## Cargo features

No Cargo features are required for Rust APIs. The `npm` feature enables the
N-API bindings used to build the Node.js package.

Rust code should depend on the crate without a feature list:

```toml
peisar = "2.0.1"
```

The previous `peisar::parsers`, `peisar::yaml`, `PeisarAst`, and `md_to_ast`
exports are no longer available. Block and inline node types are re-exported
from `peisar::ast`; use `Document::parse` or `md_to_ast_with_hooks` instead.
HTML rendering and front matter are available through the high-level
`Peisar` API. See the [Rust overview](./README.md) for end-to-end usage.
