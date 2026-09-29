<!-- markdownlint-disable MD033 -->
<!-- markdownlint-disable MD041 -->
<div align="center">
<img src="https://pub-c9ba018358dd48a99b70013b65a25e5f.r2.dev/logo/peisar.webp" width="160" height="160" alt="Peisar logo" />
  <h1>Peisar</h1>
  <p>A Rust Markdown parser and HTML renderer with native Node.js bindings.</p>
</div>

---

[![NPM](https://nodei.co/npm/peisar.svg)](https://nodei.co/npm/peisar/)

Peisar parses CommonMark Markdown with GitHub Flavored Markdown (GFM),
Kramdown block attributes, YAML front matter, source spans, AST visitors, and
custom parser hooks. The crate also contains an in-memory Markdown and asset
cache plus a `Peisar.toml` static-site configuration loader.

## Features

- CommonMark blocks and inlines, including HTML and reference links
- GFM tables, strikethrough, task lists, and autolinks
- Kramdown block attributes such as `{:#intro .lead key="value"}`
- YAML front matter, returned separately from rendered Markdown
- Source spans on every AST node
- AST visitors and during-parse extension hooks
- A native N-API Node.js addon with generated TypeScript declarations
- A Rust library (`rlib`) with public module paths for parser, renderer, cache,
  and SSG configuration APIs

## Node.js

Install the package:

```sh
npm install peisar
```

Create a `Peisar` document, then read its AST, rendered HTML, front matter, or
serialized AST:

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello **world**", { fragment: true });

console.log(document.html); // <h1>Hello <strong>world</strong></h1>\n
console.log(document.ast); // { nodeType: "root", children: [...], ... }
console.log(document.astJson); // JSON string
console.log(document.frontmatter); // object, or null
```

### Options

All options are optional. GFM and Kramdown are enabled by default; `html`
returns a complete document unless `fragment` is `true`.

```js
const document = new Peisar("# Title", {
  gfm: false,
  kramdown: false,
  fileName: "guide.md",
  fragment: true,
  charset: true,
  viewport: true,
  title: "Guide",
  bodyClass: "docs",
  style: "h1 { color: rebeccapurple; }",
});
```

`charset`, `viewport`, `title`, `bodyClass`, and `style` affect full-document
output only.

### Front matter

Leading YAML front matter is removed before Markdown parsing and exposed as a
JavaScript object:

```js
const document = new Peisar(`---
title: Hello
tags:
  - docs
---

# Hello`, { fragment: true });

console.log(document.frontmatter); // { title: "Hello", tags: ["docs"] }
console.log(document.html); // <h1>Hello</h1>\n
```

### AST visitors

Register visitors with `useVisitor`. Each callback receives a one-item tuple,
may mutate the supplied node, and can return a control object. Return
`{ recurse: true }` to visit children; replacements are not visited again.

```js
const document = new Peisar("# Hello", { fragment: true });

document.useVisitor({
  visitBlock([block]) {
    if (block.type === "Heading") {
      return { replaceWith: [{ ...block, level: 2 }] };
    }
    return { recurse: true };
  },
});

console.log(document.html); // <h2>Hello</h2>\n
```

The control fields are `insertBefore`, `insertAfter`, `replaceWith`, `remove`,
and `recurse`. The same shape applies to `visitInline`.

### Custom parser hooks

Use `useParser` for syntax that must be recognized while parsing. Block hooks
receive `{ line, lineIndex, lines }`; inline hooks receive `{ rest, index }`.
Hooks run before built-in matchers, in registration order. Return `undefined`
to decline a position.

```js
const document = new Peisar("See [[Some Page]].", { fragment: true });
const zero = { line: 0, column: 0, offset: 0 };

document.useParser({
  parseInline([{ rest }]) {
    if (!rest.startsWith("[[")) return;
    const close = rest.indexOf("]]");
    if (close === -1) return;
    const target = rest.slice(2, close);
    return {
      inline: {
        type: "Link",
        text: [{ type: "Text", value: target, pos: { start: zero, end: zero } }],
        url: `/wiki/${target.replaceAll(" ", "_")}`,
        autolink: false,
        pos: { start: zero, end: zero },
      },
      consumed: close + 2,
    };
  },
});

console.log(document.html); // <p>See <a href="/wiki/Some_Page">Some Page</a>.</p>\n
```

`parseBlock` must consume at least one line. `parseInline` must consume at
least one character; a missing or zero `consumed` value declines the result.
Peisar recomputes the source span of a hook-returned node, so a complete
placeholder `pos` is sufficient.

### Cache and SSG configuration

`PeisarCache` loads Markdown and asset files from an entry directory, exposes
their absolute paths and contents, and can watch for changes. Call `dispose()`
when a watcher is no longer needed.

```js
const { PeisarCache } = require("peisar");

const cache = new PeisarCache("contents", "public");
console.log(cache.markdownFiles());
console.log(cache.getText("/absolute/path/to/contents/index.md"));
cache.startWatchingJs();
cache.dispose();
```

`peisarSsgConfig()` reads `Peisar.toml` from the current working directory.
It returns resolved defaults, but reports an invalid or missing configuration
to stderr and exits the host process. Use it only where that fatal behavior is
appropriate.

```toml
[site]
title = "My Site"

[directories]
outDir = "build"

[devServer]
port = 3838
host = "127.0.0.1"
```

```js
const { peisarSsgConfig } = require("peisar");
console.log(peisarSsgConfig().directories.outDir); // "build"
```

The directory defaults are `rootDir = "."`, `contentsDir = "contents"`,
`publicDir = "public"`, `themeDir = "themes"`, and `outDir = ".peisar"`.
The development-server defaults are `127.0.0.1:3838`.

## Rust

Add the crate to a Rust project:

```toml
[dependencies]
peisar = "0.1.2"
```

The public Rust API is module-based. Import the parser from
`peisar::markdown::ast`, the renderer from `peisar::markdown::html`, and the
high-level document wrapper from `peisar::markdown::peisar`.

### Parse and render

```rust
use peisar::markdown::ast::{AstOptions, Document};
use peisar::markdown::ast::tokens::token::Block;
use peisar::markdown::html::{RenderOptions, render_document_html};

let document = Document::parse("# Hello **world**\n", &AstOptions::default(), None);
assert!(matches!(document.children[0], Block::Heading { level: 1, .. }));

let html = render_document_html(
    &document,
    Some(RenderOptions {
        fragment: true,
        ..Default::default()
    }),
);
assert_eq!(html, "<h1>Hello <strong>world</strong></h1>\n");
```

For the Node-like parse-and-render wrapper:

```rust
use peisar::markdown::config::PeisarOptions;
use peisar::markdown::peisar::Peisar;

let mut document = Peisar::new(
    "# Hello".into(),
    Some(PeisarOptions {
        fragment: Some(true),
        ..Default::default()
    }),
);
assert_eq!(document.html(), "<h1>Hello</h1>\n");
```

### Front matter and visitors

```rust
use peisar::frontmatter::parse_markdown_frontmatter;

let parsed = parse_markdown_frontmatter("---\ntitle: Hello\n---\n\n# Hello")?;
assert_eq!(parsed.yaml_data().unwrap()["title"], "Hello");
```

For post-parse traversal, implement
`peisar::markdown::ast::AstVisitor` and call
`peisar::markdown::ast::visitor::visit_document_mut`. The visitor controls
are `VisitControl` and `InlineVisitControl` from that same `visitor` module.

`AstParser` and `ParseHooks` are publicly exported from
`peisar::markdown::ast` for integration with Peisar internals, but the
hook-enabled parser entry point is not yet public to external Rust consumers.
Use `useParser` from Node.js for registered custom syntax today.

The SSG configuration parser is available as
`peisar::ssg::ssg_config::parse_config`; use
`peisar::ssg::ssg_config::load_config` for the fallible current-directory
loader. The cache type is `peisar::ssg::cache::PeisarCache`.

## Development

```sh
npm install
npm run build:local
npm test
cargo test
```

Run `npm run build:local` after changing N-API-annotated Rust code; it
regenerates the package loader and TypeScript declarations before the Node.js
tests run.

## AI integration

Peisar supplies Agent Skills for AI coding tools:

```sh
npx skills add phothinmg/peisar
```

- **peisar-parse-markdown**: consume the Node.js and Rust Markdown APIs
- **peisar-ast-visitors**: make post-parse AST transformations
- **peisar-parser-hooks**: add syntax recognized during parsing
- **peisar-development**: modify and validate the codebase

## Project structure

```text
src/
├── frontmatter/             YAML front-matter parser
├── markdown/
│   ├── ast/                 AST nodes, parsers, visitors, and hooks
│   ├── config/              combined parser and renderer options
│   ├── html/                HTML renderer
│   └── peisar/              high-level N-API `Peisar` class
├── ssg/
│   ├── cache/               file cache and watcher
│   └── ssg_config/          `Peisar.toml` parser and loader
└── lib.rs                   public module declarations
```

The generated Node.js declaration file is `index.d.ts` in published builds.
Run `cargo doc --no-deps --open` for Rust API documentation.
