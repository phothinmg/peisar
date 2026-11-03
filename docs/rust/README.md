# Rust usage

Peisar provides a high-level Rust parser wrapper and optional lower-level
AST, HTML, plugin, and YAML APIs. The implementation is split between this
crate and its `peisar_ast` / `peisar_yaml` dependencies.

## Install

```toml
[dependencies]
peisar = "2.0.0"
```

The high-level `Peisar` wrapper is available without optional features. Enable
the feature for each lower-level module you use:

| Cargo feature | Public module | Purpose |
| --- | --- | --- |
| `ast` | `peisar::ast` | AST types and parser helpers |
| `html` | `peisar::html` | HTML rendering |
| `plugin` | `peisar::plugin` | Plugin types and traits |
| `yaml` | `peisar::yaml` | Front matter and standalone YAML helpers |

For example:

```toml
peisar = { version = "2.0.0", features = ["ast", "html", "yaml"] }
```

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

For lower-level parsing and rendering, enable `ast` and `html`:

```rust
use peisar::ast::{AstOptions, Document};
use peisar::html::{RenderOptions, render_document_html};

let document = Document::parse("# Hello\n", &AstOptions::default(), None);
let html = render_document_html(
    &document,
    Some(RenderOptions {
        fragment: true,
        ..RenderOptions::default()
    }),
);
assert_eq!(html, "<h1>Hello</h1>\n");
```

## Front matter and YAML

Enable the `yaml` feature to use the re-exported helpers:

```rust
use peisar::yaml::frontmatter;

let parsed = frontmatter("---\ntitle: Hello\n---\n\n# Hello".to_string())?;
assert_eq!(parsed.yaml_data().unwrap()["title"], "Hello");
assert_eq!(parsed.clean_content(), "# Hello");
# Ok::<(), String>(())
```

See the [Rust API reference](./apis.md) for module details and links to the
Node.js guides.
