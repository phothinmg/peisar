---
name: peisar-parse-markdown
description: Parse Markdown and render HTML with Peisar's native Node.js addon or Rust library. Use for HTML rendering, AST access, YAML front matter, parser options.
---

# Parse Markdown with Peisar

Peisar supports CommonMark, GFM, Kramdown block attributes, YAML front
matter, source spans, and HTML rendering. Its Node.js API is a native N-API
addon; Rust consumers use the public module paths under `peisar::markdown`.

## Node.js

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello **world**", { fragment: true });
console.log(document.html); // <h1>Hello <strong>world</strong></h1>\n
console.log(document.ast);
console.log(document.astJson);
console.log(document.frontmatter); // object or null
```

`ast` (the real JavaScript object) uses PascalCase `type` values
(`Heading`, `Paragraph`, `Link`) and a `nodeType: "root"` root.
`astJson` serializes the Rust AST, so its types are snake_case
(`heading`, `paragraph`, `link`) and the root uses `type: "root"`.
GFM task state is `0` (unchecked) / `1` (checked) in both shapes.

`Peisar` accepts optional `gfm`, `kramdown`, `fileName`, `fragment`,
`charset`, `viewport`, `title`, `bodyClass`, and `style` properties. GFM and
Kramdown default to `true`; `fragment` defaults to `false`. Full-document
settings (`charset`, `viewport`, `title`, `bodyClass`, and `style`) have no
effect in fragment mode.

Leading YAML front matter is removed before parsing:

```js
const document = new Peisar(
  `---
title: Guide
---

# Guide`,
  { fragment: true },
);

console.log(document.frontmatter); // { title: "Guide" }
```

AST nodes use PascalCase `type` values in Node.js (`Heading`, `Paragraph`,
`Link`, and so on). Every node has a half-open `pos` span with zero-based
`line`, `column`, and byte `offset`. The root uses `nodeType: "root"`.

For syntax extensions, use `document.useParser(...)`; for post-parse changes,
use `document.useVisitor(...)`. Register both before relying on a document
read: parser registration reparses the original source, and visitors run when
`ast`, `astJson`, `html`, or `frontmatter` is read.

## Rust

The Rust API is module-based; do not use old root imports such as
`peisar::{Document, Peisar}`.

```rust
use peisar::markdown::ast::{AstOptions, Document};
use peisar::markdown::html::{RenderOptions, render_document_html};

let document = Document::parse("# Hello\n", &AstOptions::default(), None);
let html = render_document_html(
    &document,
    Some(RenderOptions {
        fragment: true,
        ..Default::default()
    }),
);
assert_eq!(html, "<h1>Hello</h1>\n");
```

Use these paths:

- `peisar::markdown::ast::{AstOptions, Document, PeisarAst, AstVisitor}`
- `peisar::markdown::ast::{parse_inline, parse_inline_with_refs, LinkRefMap}`
- `peisar::markdown::ast::tokens::token::{Block, Inline}`
- `peisar::markdown::ast::tokens::span::{Span, Position}`
- `peisar::markdown::ast::visitor::{visit_document_mut, VisitControl, InlineVisitControl}`
- `peisar::markdown::html::{render_document_html, RenderOptions}`
- `peisar::markdown::peisar::Peisar`
- `peisar::markdown::config::PeisarOptions`
- `peisar::frontmatter::parse_markdown_frontmatter`

`parse_markdown_frontmatter` returns a `ParseResult`. Use
`pure_markdown_content()`, `yaml_data()`, or `into_parts()` to access it.

## Output details

- Fragment HTML ends in `\n`.
- `Document::parse` accepts Markdown without front matter; strip it first with
  `parse_markdown_frontmatter` when needed.
- `render_document_html(doc, None)` uses `RenderOptions::default()`, which is
  fragment mode. The `Peisar` wrapper instead defaults to a complete document.
