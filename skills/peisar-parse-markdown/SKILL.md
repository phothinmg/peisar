---
name: peisar-parse-markdown
description: Parse Markdown and render HTML with Peisar's native Node.js addon or Rust library. Use for HTML rendering, AST access, raw YAML/TOML/JSON frontmatter, and parser options.
---

# Parse Markdown with Peisar

Peisar supports CommonMark, GFM, Kramdown block attributes, YAML/TOML/JSON
frontmatter extraction, source spans, and HTML rendering. Frontmatter is
returned as raw text; Peisar does not deserialize it.

## Node.js

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello **world**", { fragment: true });
console.log(document.html);
console.log(document.ast);
console.log(document.frontmatter);
```

`Peisar` accepts optional GFM, Kramdown, file-name, and rendering options.
Use `document.use(plugin)` to register parser hooks or AST visitors.

`document.frontmatter` contains `frontmatterType` (`"yaml"`, `"toml"`,
`"json"`, or `"empty"`) and the raw frontmatter text in `raw`. Parse the
content with an application-level YAML, TOML, or JSON library if needed.

## Rust

The high-level wrapper and options are available from the crate root:

```rust
use peisar::{Peisar, PeisarOptions};

let mut document = Peisar::new(
    "# Hello\n".into(),
    Some(PeisarOptions {
        fragment: Some(true),
        ..Default::default()
    }),
);
assert_eq!(document.html(), "<h1>Hello</h1>\n");
```

The lower-level Rust APIs are available without Cargo features:

- `peisar::ast` for AST types and parser helpers
- `peisar::render` for HTML rendering
- `peisar::plugin` for plugin types and visitors

`Peisar::frontmatter()` returns a `Frontmatter` value with a format enum and
raw content. The `npm` Cargo feature is only needed when building the Node.js
bindings.

See the [Rust API reference](../../docs/rust/apis.md) and
[Node.js API reference](../../docs/nodejs/apis.md) for details.
