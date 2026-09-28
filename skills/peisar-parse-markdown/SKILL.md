---
name: peisar-parse-markdown
description: Parse Markdown and render HTML with the peisar npm package or Rust crate (CommonMark + GFM + Kramdown + YAML front matter). Use when writing code that consumes peisar, rendering Markdown to HTML, reading the AST, or extracting front matter.
---

# Parse Markdown with Peisar

Peisar parses CommonMark Markdown with GitHub Flavored Markdown (GFM), Kramdown-style block attributes, and YAML front matter. It ships as a native Node.js addon and as a plain Rust crate.

## When to Use

- Rendering Markdown to HTML in a Node.js or Rust project
- Reading a typed Markdown AST (with source spans) instead of regex-scraping
- Extracting YAML front matter from Markdown files
- Deciding which options to pass (`gfm`, `kramdown`, `fragment`, document wrappers)

## Node.js

```sh
npm install peisar
```

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello **world**");

console.log(document.html); // full HTML document by default
console.log(document.ast); // AST object (visitors applied)
console.log(document.astJson); // AST serialized as JSON
console.log(document.frontmatter); // deserialized YAML, or null
```

### Options

Only pass what you need; omitted options keep defaults (`gfm: true`, `kramdown: true`, `fragment: false`):

```js
const document = new Peisar("# Title", {
  gfm: false, // disable tables, strikethrough, task lists, autolinks
  kramdown: false, // disable `{:#id .class key="value"}` block attributes
  fileName: "doc.md", // attached to the AST
  fragment: true, // render only body content (no <html> wrapper)
  charset: true, // <meta charset> (full-document mode only)
  viewport: true, // <meta viewport> (full-document mode only)
  title: "My Page", // <title> (full-document mode only)
  bodyClass: "docs", // extra classes on <body>
  style: "h1{color:red}", // inline <style> in head
});
```

`fragment: true` is what you want for embedding output into your own page:

```js
new Peisar("# Title", { fragment: true }).html; // "<h1>Title</h1>\n"
```

### Front matter

```js
const document = new Peisar(`---
title: Hello
tags:
  - one
---

# Hello`);

document.frontmatter; // { title: 'Hello', tags: ['one'] }
document.html; // front matter stripped; "<h1>Hello</h1>\n" with fragment
```

### AST shape

`document.ast` is `{ type: 'root', children: [...], linkReferences: [] }` (plus `fileName`/`pos`). Block nodes: `Heading` (`level`, `children`), `Paragraph`, `CodeBlock` (`lang`, `code`), `BlockQuote`, `List` (`ordered`, `items`), `Table`, `ThematicBreak`, `HtmlBlock`, `LinkReferenceDefinition`, `Comment`. Inline nodes: `Text`, `Emphasis` (`level: 1|2`), `Code`, `HtmlInline`, `Strikethrough`, `HardBreak`, `SoftBreak`, `Image` (`alt`, `url`, `title`), `Link` (`text`, `url`, `title`, `autolink`), `LinkReference`.

Every node carries `pos: { start, end }` with zero-based `{ line, column, offset }` source positions. Optional `attrs` on blocks holds parsed Kramdown attributes (`id`, `classes`, `attributes`).

### Register before reading

The document is parsed at construction; `useVisitor` and `useParser` must be called **before** first access to `ast`, `html`, `frontmatter`, or `astJson` (parser hooks trigger a re-parse, but reading first means the AST you saw is stale).

## Rust

```toml
[dependencies]
peisar = "0.1.2"
```

```rust
use peisar::{AstOptions, Document};

let doc = Document::parse("# Hello **world**\n", &AstOptions::default(), None);
assert_eq!(doc.node_type, "root");
```

Strict CommonMark with a file name:

```rust
let opts = AstOptions {
    gfm: false,
    kramdown: false,
    file_name: Some("doc.md".into()),
};
let doc = Document::parse("# Title\n", &opts, opts.file_name.clone());
```

### Parse + render (`Peisar` mirrors the JS class)

```rust
use peisar::{Peisar, PeisarOptions};

let mut fragment = Peisar::new(
    "# Hello **world**".into(),
    Some(PeisarOptions { fragment: Some(true), ..Default::default() }),
);
println!("{}", fragment.html()); // <h1>Hello <strong>world</strong></h1>
```

### Front matter standalone

```rust
use peisar::frontmatter::parse_markdown_frontmatter;

let parsed = parse_markdown_frontmatter("---\ntitle: Hello\n---\n\n# Hello")?;
let md: &str = parsed.pure_markdown_content(); // "# Hello"
let yaml = parsed.yaml_data().unwrap();         // {"title": "Hello"}
let (md, yaml) = parsed.into_parts();           // or take both by value
```

### Inline-only parsing

`parse_inline` parses inline content in isolation (headings, table cells, plain text). `parse_inline_with_refs` additionally resolves reference-style links against a `LinkRefMap`:

```rust
use peisar::{LinkRefMap, parse_inline_with_refs};

let mut refs = LinkRefMap::new();
refs.insert("example".into(), ("https://example.com".into(), None));

let inlines = parse_inline_with_refs("see [example][]", None, Some(&refs));
```

## Rendering Defaults

- Fragment mode output ends with a trailing newline (e.g. `"<h1>Hi</h1>\n"`)
- Full-document mode wraps body content in `<!DOCTYPE html>`, `<html>`, `<head>` (charset, viewport, optional title/style), and `<body>`
- GFM task lists render as `disabled` checkboxes with `data-checked="true|false"` and class `task-list-item-checkbox`
- Kramdown `{:#id .class key="value"}` lines following a block become `id`/`class`/arbitrary HTML attributes

## Common Mistakes

- **Reading `html` before registering hooks/visitors** — parse happens eagerly at construction; visitors run at property access, hooks re-parse.
- **Expecting `frontmatter` to appear in `html`** — it is stripped from rendering and exposed separately.
- **Assuming `fragment` defaults to `true`** — it defaults to `false` (full document) in the JS API.
- **Comparing HTML without the trailing `\n`** — rendered fragments include it.
