# Rust usage

Peisar provides a high-level Rust parser wrapper and lower-level AST, render,
and plugin APIs. These modules are available without Cargo feature flags.

## Install

```toml
[dependencies]
peisar = "2.1.0"
```

No Cargo features are needed when using Peisar from Rust. The `npm` feature
only enables N-API bindings for the Node.js package.

The public Rust API includes:

| Public API | Purpose |
| --- | --- |
| `peisar::{Peisar, PeisarOptions}` | High-level parsing, HTML rendering, and front matter |
| `peisar::{Frontmatter, FrontmatterType}` | Detected frontmatter format and raw content |
| `peisar::ast` | AST types and parser helpers |
| `peisar::render` | HTML renderer and render options |
| `peisar::plugin` | Plugin types, traits, and visitor helpers |

Frontmatter is extracted as raw content; it is not deserialized as YAML,
TOML, or JSON.

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
use peisar::ast::PeisarAst;

let mut document = PeisarAst::new("# Hello\n".into(), None);
assert_eq!(document.get_ast().children.len(), 1);
```

Use the high-level wrapper for HTML and front matter:

```rust
let mut document = Peisar::new("---\ntitle: Hello\n---\n\n# Hello".into(), None);
let frontmatter = document.frontmatter();
assert_eq!(frontmatter.frontmatter_type, peisar::FrontmatterType::Yaml);
assert_eq!(frontmatter.raw, "title: Hello");
assert_eq!(document.html(), "<h1>Hello</h1>\n");
```

See the [Rust API reference](./apis.md) for module details and links to the
Node.js guides.
