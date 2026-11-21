# Quick Start

Peisar parses a Markdown string and exposes its abstract syntax tree (AST),
rendered HTML, and frontmatter. Choose the [Node.js](#nodejs) or
[Rust](#rust) example to get started.

## Node.js

Install the package:

```sh
npm install peisar
```

Parse and render a Markdown document:

```js
import { Peisar } from "peisar";

const document = new Peisar("# Hello **world**", { fragment: true });

console.log(document.html);
// <h1>Hello <strong>world</strong></h1>
console.log(document.ast);
```

`fragment: true` returns only the rendered content. Without it, Peisar
generates a complete HTML document. See the [Node.js overview](docs/nodejs/README.md)
for frontmatter and plugin examples.

## Rust

Add Peisar to your `Cargo.toml`:

```toml
[dependencies]
peisar = "2.1.0"
```

Then parse and render Markdown:

```rust
use peisar::{Peisar, PeisarOptions};

let mut document = Peisar::new(
    "# Hello **world**\n".into(),
    Some(PeisarOptions {
        fragment: Some(true),
        ..Default::default()
    }),
);

println!("{}", document.html());
```

See the [Rust overview](docs/rust/README.md) for lower-level AST access and
frontmatter.

## Next steps

- [Node.js guides](docs/nodejs/README.md)
- [Rust guides](docs/rust/README.md)
- [Markdown language specification](docs/markdown-spec.md)
