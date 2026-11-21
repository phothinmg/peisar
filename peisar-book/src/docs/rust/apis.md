# Rust API reference

The crate re-exports `Peisar`, `PeisarOptions`, and frontmatter types
(`Frontmatter`, `FrontmatterResult`, and `FrontmatterType`) at the root. The
`ast`, `render`, and `plugin` modules are public without feature flags.

## `peisar::{Peisar, PeisarOptions}`

`Peisar` is the high-level single-document parser and renderer. It exposes
`ast()`, `html()`, `frontmatter()`, and `use_plugin()` methods.
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

`frontmatter()` returns a `Frontmatter` value with `frontmatter_type` and
`raw` fields. The content is not deserialized.

## `peisar::ast`

`PeisarAst`, the AST types, and parsing helpers are available from this module.
`md_to_ast` parses Markdown directly into a `PeisarDocument`; the input should
not include frontmatter if you want it extracted separately.

Use `PeisarAst` for parsing with frontmatter extraction and optional plugins,
or `md_to_ast` for direct AST parsing.

```rust
use peisar::ast::PeisarAst;

let mut document = PeisarAst::new("# Hello\n".into(), None);
let ast = document.get_ast();
assert_eq!(ast.children.len(), 1);
```

## `peisar::render`

This module provides `AstToHtml`, `ast_to_html`, and `RenderOptions` for
rendering AST documents directly.

## `peisar::plugin`

This module provides the plugin interfaces and visitor helpers. It is
available without enabling a Cargo feature. `PeisarPlugin` contains optional
`visit_block`, `visit_inline`, `parse_block`, and `parse_inline` callbacks.
Register it with `Peisar::use_plugin()` or `PeisarAst::add_plugin()`. See the
[Rust plugin example](https://github.com/phothinmg/peisar/blob/main/examples/rust/plugin.rs)
for visitor and parser hook examples.

The `ast` module also exposes token types through `peisar::ast::tokens`, source
positions through `peisar::ast::span`, and parser helpers through
`peisar::ast::parsers`.

## Cargo features

No Cargo features are required for Rust APIs. The `npm` feature enables the
N-API bindings used to build the Node.js package.

Rust code should depend on the crate without a feature list:

```toml
peisar = "2"
```

The `npm` feature enables the N-API bindings used to build the Node.js
package. See the [Rust overview](./README.md) for end-to-end usage.
