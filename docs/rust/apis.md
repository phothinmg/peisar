# Rust API reference

The crate re-exports `Peisar`, `PeisarOptions`, and frontmatter types at the
root. The `ast`, `render`, and `plugin` modules are public without feature
flags.

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

The AST types and parsing helpers are available from this module.

Use `PeisarAst` for parsing and accessing the AST, or `md_to_ast` for direct
parsing. Custom parser hooks are available through the plugin APIs.

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
available without enabling a Cargo feature. Register a `Plugin` with
`Peisar::use_plugin()` or `PeisarAst::add_plugin()`.

## Cargo features

No Cargo features are required for Rust APIs. The `npm` feature enables the
N-API bindings used to build the Node.js package.

Rust code should depend on the crate without a feature list:

```toml
peisar = "2"
```

The `npm` feature enables the N-API bindings used to build the Node.js
package. See the [Rust overview](./README.md) for end-to-end usage.
