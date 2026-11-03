# Rust API reference

The crate re-exports `Peisar`, `PeisarOptions`, and `get_options` at the root.
The lower-level APIs are split into optional, feature-gated modules.

## `peisar::{Peisar, PeisarOptions}`

`Peisar` is the high-level single-document parser and renderer. It exposes
`ast()`, `html()`, `frontmatter()`, and `ast_json()` methods. `PeisarOptions`
configures GFM, Kramdown attributes, file metadata, and HTML output.

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

## Feature-gated modules

Enable each Cargo feature to expose its corresponding module. The crate's
default feature set is empty.

| Feature | Module | Main exports |
| --- | --- | --- |
| `ast` | `peisar::ast` | `AstOptions`, `Document`, AST node types, parse and visitor helpers |
| `html` | `peisar::html` | `RenderOptions`, `render_document_html`, `AstToHtml` |
| `plugin` | `peisar::plugin` | `Plugin`, `PluginFactory`, `RegisteredPlugin` |
| `yaml` | `peisar::yaml` | `FrontmatterResult`, `frontmatter`, `load_yaml`, `yaml_parser` |

### `peisar::ast`

Use `Document::parse` to parse Markdown directly. This module re-exports AST
types and parser helpers from `peisar_ast`.

```rust
use peisar::ast::{AstOptions, Document};

let document = Document::parse("# Hello\n", &AstOptions::default(), None);
assert_eq!(document.children.len(), 1);
```

### `peisar::html`

`render_document_html` converts a parsed document to HTML. Pass
`RenderOptions { fragment: true, .. }` to render only the body. The default
render options use fragment mode.

### `peisar::plugin`

This module re-exports the plugin interfaces from `peisar_ast`. Enable both
`ast` and `plugin` when using plugin types alongside AST parsing.

### `peisar::yaml`

`frontmatter` separates leading YAML front matter from Markdown content.
`yaml_parser` parses YAML text, while `load_yaml` reads and parses a YAML file.

## Cargo features

```toml
peisar = { version = "2.0.0", features = ["ast", "html", "plugin", "yaml"] }
```

All features are optional and disabled by default. See the [Rust overview](./README.md)
for an installation example and end-to-end usage.
