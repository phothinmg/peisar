<!-- markdownlint-disable MD033 -->
<!-- markdownlint-disable MD041 -->
<div align="center">
<img src="https://pub-c9ba018358dd48a99b70013b65a25e5f.r2.dev/logo/peisar.webp" width="160" height="160" alt="peisar" />
  <h1>Peisar</h1>
  <p>A lossless Markdown toolchain written in Rust — parse, transform, and render with source-span precision.</p>
</div>

---

A practical Markdown parser written in Rust, supporting CommonMark,
GitHub Flavored Markdown (GFM), Kramdown-style block attributes, a plugin
system, and configurable HTML output.

## Features

- **CommonMark** — headings, paragraphs, code blocks, block quotes, lists,
  thematic breaks, HTML blocks, emphasis, links, images, inline code,
  hard/soft breaks.
- **GFM** — tables (with column alignment), strikethroughs, task lists,
  autolinks.
- **Kramdown attributes** — `{:#id .class key="value"}` on any block element.
- **Plugin system** — extend the parser/AST/renderer via `Plugin` trait.
- **Configurable** — full HTML document or fragment, custom title/CSS.
- **Zero dependencies** — only `serde` for serialization.

## Quick start

```rust
use peisar::{parser, html, config::{ParseOptions, RenderOptions}};

// Parse with default options (GFM + Kramdown enabled)
let doc = parser::parse("# Hello **world**");

// Render as a full HTML document
let html = html::render_document(&doc, Some(false));
```

## Node.js WASI build

Build the threaded WASI binding with:

```sh
rustup target add wasm32-wasip1-threads
npm run build:wasi
```

The generated binding uses Node's WASI and worker-thread APIs, so use it in
Node.js rather than instantiating the `.wasm` file directly in a browser. Keep
[`index.js`](./index.js), [`peisar.wasi.cjs`](./peisar.wasi.cjs), and
[`peisar.wasm32-wasi.wasm`](./peisar.wasm32-wasi.wasm) together when deploying
the package.

To require the WASI binding instead of a native addon:

```sh
NAPI_RS_FORCE_WASI=error node app.js
```

```js
const { Peisar } = require('peisar')

const parser = new Peisar('# Hello from WASI')
console.log(parser.html)
```

## Configuration

### Parse options

```rust
use peisar::config::ParseOptions;

// Pure CommonMark (no GFM, no Kramdown)
let opts = ParseOptions { gfm: false, kramdown: false };
let doc = peisar::parser::parse_with("# Title", &opts);
```

### Render options

```rust
use peisar::config::RenderOptions;

let opts = RenderOptions {
    fragment: false,                          // full HTML document
    charset: true,
    viewport: true,
    title: Some("My Page".to_string()),
    body_class: Some("markdown-body".to_string()),
    style: Some("body { max-width: 800px; }".to_string()),
};
```

## Plugins

```rust
use peisar::plugin_factory::{Plugin, PluginPipeline, PluginContext};
use peisar::plugins::WrapDiv;

let doc = peisar::parser::parse("# Hello");

let mut pipeline = PluginPipeline::new();
pipeline.add(WrapDiv { class: "container".to_string() });

let ctx = PluginContext::new(ParseOptions::default());
let html = peisar::html::render_document_with(
    &doc,
    &RenderOptions::default(),
    &pipeline,
    &ctx,
);
```

### Writing a custom plugin

```rust
use peisar::plugin_factory::{Plugin, PluginContext};
use peisar::ast::{Block, Document, KramdownAttributes};

struct AddHeadingClass { class: String }

impl Plugin for AddHeadingClass {
    fn name(&self) -> &str { "add-heading-class" }

    fn transform_ast(&self, doc: &mut Document, _ctx: &PluginContext) {
        for block in &mut doc.children {
            if let Block::Heading { attrs, .. } = block {
                let mut a = attrs.take().unwrap_or_default();
                a.classes.push(self.class.clone());
                *attrs = Some(a);
            }
        }
    }
}
```

## Kramdown attributes

```markdown
# Heading {#my-id .big .red data-toggle="modal"}
```

code block

```
{#code-id .highlight}
```

## GFM examples

### Tables

```markdown
| Left | Center | Right |
| :--- | :----: | ----: |
| a    |   b    |     c |
```

### Task lists

```markdown
- [x] Done
- [ ] Todo
```

### Strikethrough

```markdown
~~deleted text~~
```

## Project structure

```
src/
├── ast/       — AST node definitions (mod, nodes, span)
├── parser/    — Parser modules (block, inline, gfm, kramdown)
├── config.rs  — ParseOptions, RenderOptions
├── plugin.rs  — Plugin system
├── html.rs    — HTML renderer
├── visitor.rs — Visitor pattern
└── tests.rs   — Test suite
```

See [CHANGELOG.md](CHANGELOG.md) for release history and [NOTES.md](NOTES.md)
for design decisions and future plans.
