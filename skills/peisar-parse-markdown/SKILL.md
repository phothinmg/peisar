---
name: peisar-parse-markdown
description: Parse Markdown and render HTML with Peisar's native Node.js addon or Rust library. Use for HTML rendering, AST access, YAML front matter, and parser options.
---

# Parse Markdown with Peisar

Peisar supports CommonMark, GFM, Kramdown block attributes, YAML front
matter, source spans, and HTML rendering.

## Node.js

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello **world**", { fragment: true });
console.log(document.html);
console.log(document.ast);
console.log(document.astJson);
console.log(document.frontmatter);
```

`Peisar` accepts optional GFM, Kramdown, file-name, and rendering options.
Use `document.use(plugin)` to register parser hooks or AST visitors.

The package also exports standalone YAML helpers:

```js
const { frontmatter, yamlParser, loadYaml } = require("peisar");

// Split front matter from Markdown.
const { cleanContent, yamlData } = frontmatter(
  "---\ntitle: Hello\n---\n\n# Hello",
);

// Parse a YAML string.
const config = yamlParser("name: config\nitems:\n  - one\n");

// Read and parse a YAML file (relative to the process working directory).
const data = loadYaml("config.yml");
```

`frontmatter` returns `{ cleanContent, yamlData }`; `yamlData` is `null`
when no front matter exists.

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
- `peisar::plugin` for plugin types and visitors

Use the high-level `Peisar` wrapper for HTML rendering and front matter.
The `npm` Cargo feature is only needed when building the Node.js bindings.

See the [Rust API reference](../../docs/rust/apis.md) and
[Node.js API reference](../../docs/nodejs/apis.md) for details.
