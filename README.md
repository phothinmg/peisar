<!-- markdownlint-disable MD033 -->
<!-- markdownlint-disable MD041 -->
<div align="center">
  <h1>Peisar</h1>
  <p>A practical Markdown parser written in Rust</p>
</div>


![NPM Version](https://img.shields.io/npm/v/peisar?logo=npm&logoColor=%23CB3837) ![NPM Downloads](https://img.shields.io/npm/dm/peisar?logo=npm&logoColor=%23CB3837)
![Crates.io Version](https://img.shields.io/crates/v/peisar?logo=rust) [![Documentation][docs_img]][docs]


---

Peisar parses CommonMark Markdown with GitHub Flavored Markdown (GFM),
Kramdown block attributes, YAML front matter, source spans, AST visitors,
custom parser hooks, and a Rust-first architecture that is also exposed to
Node.js through N-API.

> [!NOTE]
> The README is intentionally short so the npm and crates.io pages stay easy
> to scan. Detailed guides now live in the repository under `docs/`, which
> lets the project update documentation between package releases.

## Features

- CommonMark blocks and inlines, including HTML and reference links
- GFM tables, strikethrough, task lists, and autolinks
- Kramdown block attributes such as `{:#intro .lead key="value"}`
- YAML front matter extraction
- AST visitors and parser hooks through `usePlugin()`
- Native Node.js addon and Rust library surfaces

## Install

### Node.js

```sh
npm i peisar
```

### Rust

```toml
[dependencies]
peisar = "1.0.0"
```

## Quick start

### Node.js

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello **world**", { fragment: true });

console.log(document.html);
console.log(document.ast);
console.log(document.frontmatter);
```

### Rust

```rust
use peisar::markdown::ast::{AstOptions, Document};
use peisar::markdown::html::{RenderOptions, render_document_html};

let doc = Document::parse("# Hello **world**\n", &AstOptions::default(), None);
let html = render_document_html(
    &doc,
    Some(RenderOptions {
        fragment: true,
        ..RenderOptions::default()
    }),
);

assert_eq!(html, "<h1>Hello <strong>world</strong></h1>\n");
```

## Documentation

- [Documentation hub][docs-hub]

### Node.js usage

- [Node.js overview][nodejs-guide]
- [Next.js guide][nodejs-nextjs]
- [Vite guide][nodejs-vite]
- [WASM guide][nodejs-wasm]
- [Node.js API reference][nodejs-api]

### Browser usage

- [Browser example][browser-example]

### Rust usage

- [Rust overview][rust-guide]
- [Rust API reference][rust-api]

### Benchmarks

- [Benchmark results and methodology][benchmarks]

### Other docs

- [Markdown parsing spec][markdown-spec]
- [docs.rs API documentation][docs]

## Packages and examples

- npm: <https://www.npmjs.com/package/peisar>
- crates.io: <https://crates.io/crates/peisar>
- Next.js example: <https://github.com/phothinmg/peisar/tree/main/examples/next>
- Vite example: <https://github.com/phothinmg/peisar/tree/main/examples/vite>

## Development

```sh
npm install
npm run js:test
npm run build
cargo test
```

More detail in [CONTRIBUTING](CONTRIBUTING.md).

## AI integration

Peisar supplies Agent Skills for AI coding tools:

```sh
npx skills add phothinmg/peisar
```

- **peisar-parse-markdown**: consume the Node.js and Rust Markdown APIs
- **peisar-ast-visitors**: make post-parse AST transformations
- **peisar-parser-hooks**: add syntax recognized during parsing
- **peisar-development**: modify and validate the codebase

## License

[Apache-2.0][license] © [Pho Thin Maung][ptm]

Security policy: [SECURITY.md](SECURITY.md)

<!-- Links and Badges -->

[license]: LICENSE
[ptm]: https://github.com/phothinmg
[docs]: https://docs.rs/peisar "Documentation"
[docs_img]: https://docs.rs/peisar/badge.svg "Documentation"
[docs-hub]: https://github.com/phothinmg/peisar/blob/main/docs/README.md
[nodejs-guide]: https://github.com/phothinmg/peisar/blob/main/docs/nodejs/README.md
[nodejs-nextjs]: https://github.com/phothinmg/peisar/blob/main/docs/nodejs/nextjs.md
[nodejs-vite]: https://github.com/phothinmg/peisar/blob/main/docs/nodejs/vite.md
[nodejs-wasm]: https://github.com/phothinmg/peisar/blob/main/docs/nodejs/wasm.md
[nodejs-api]: https://github.com/phothinmg/peisar/blob/main/docs/nodejs/apis.md
[browser-example]: https://github.com/phothinmg/peisar/tree/main/examples/browser
[rust-guide]: https://github.com/phothinmg/peisar/blob/main/docs/rust/README.md
[rust-api]: https://github.com/phothinmg/peisar/blob/main/docs/rust/apis.md
[benchmarks]: https://github.com/phothinmg/peisar/blob/main/docs/benchmarks.md
[markdown-spec]: https://github.com/phothinmg/peisar/blob/main/docs/markdown-spec.md
