# Rust usage

Peisar provides a high-level Rust parser wrapper and lower-level AST and
plugin APIs. The `ast` and `plugin` modules are available by default.

## Install

```toml
[dependencies]
peisar = "2.0.1"
```

No Cargo features are needed when using Peisar from Rust. The `npm` feature
only enables N-API bindings for the Node.js package.

The public Rust API includes:

| Public API | Purpose |
| --- | --- |
| `peisar::{Peisar, PeisarOptions}` | High-level parsing, HTML rendering, and front matter |
| `peisar::ast` | AST types and parser helpers |
| `peisar::plugin` | Plugin types, traits, and visitor helpers |

The lower-level HTML renderer and standalone YAML helpers are not exposed as
Rust modules.

## Parse and render

The high-level wrapper is re-exported at the crate root:

```rust
use peisar::{Peisar, PeisarOptions};

let mut document = Peisar::new(
    "# Hello **world**\n".into(),
    Some(PeisarOptions {
        fragment: Some(true),
        ..Default::default()
    }),
);

assert_eq!(document.html(), "<h1>Hello <strong>world</strong></h1>\n");
```

For lower-level AST parsing:

```rust
use peisar::ast::{AstOptions, Document};

let document = Document::parse("# Hello\n", &AstOptions::default(), None);
assert_eq!(document.children.len(), 1);
```

Use the high-level wrapper for HTML and front matter:

```rust
let mut document = Peisar::new("---\ntitle: Hello\n---\n\n# Hello".into(), None);
assert_eq!(document.frontmatter().unwrap()["title"], "Hello");
assert_eq!(document.html(), "<h1>Hello</h1>\n");
```

See the [Rust API reference](./apis.md) for module details and links to the
Node.js guides.
