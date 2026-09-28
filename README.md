<!-- markdownlint-disable MD033 -->
<!-- markdownlint-disable MD041 -->
<div align="center">
<img src="https://pub-c9ba018358dd48a99b70013b65a25e5f.r2.dev/logo/peisar.webp" width="160" height="160" alt="peisar" />
  <h1>Peisar</h1>
  <p>A Markdown parser and HTML renderer for Node.js and Rust, written in Rust.</p>
</div>

---

[![NPM](https://nodei.co/npm/peisar.svg)](https://nodei.co/npm/peisar/)

---


Peisar parses CommonMark Markdown with GitHub Flavored Markdown (GFM),
Kramdown-style block attributes, YAML front matter, source spans, AST
visitors, custom parser hooks, and configurable HTML output. It ships as
a native Node.js addon **and** as a plain Rust library (`rlib`) for use
in any Rust project.

## Features

- **CommonMark** — headings, paragraphs, code blocks, block quotes, lists,
  thematic breaks, HTML blocks, emphasis, links, images, inline code, and
  hard or soft breaks.
- **GFM** — tables with column alignment, strikethrough, task lists, and
  autolinks.
- **Kramdown attributes** — `{:#id .class key="value"}` on block elements.
- **YAML front matter** — metadata is available separately from the parsed
  Markdown.
- **AST visitors** — transform the parsed AST from JavaScript or Rust
  before reading it or rendering HTML.
- **Custom parser hooks** — parse extension syntax (directives, wikilinks,
  mentions) during parsing, before the built-in matchers.
- **Source spans** — every node carries a zero-based `[start, end)` span
  (line, column, byte offset) into the original source.
- **Node.js bindings** — native N-API addon with TypeScript declarations.
- **Rust crate** — the same parser is usable as a plain Rust library,
  without Node.

## Quick start

Install the package:

```sh
npm install peisar
```

Parse Markdown and render it as HTML:

```js
const { Peisar } = require('peisar')

const document = new Peisar('# Hello **world**')

console.log(document.html)
console.log(document.ast)
console.log(document.astJson)
```

By default, GFM and Kramdown extensions are enabled and `html` is a complete
HTML document.

## Development build

```sh
npm install
npm run build:local
npm test
```

## Configuration

Pass only the options you need; omitted properties retain their defaults.

```js
const { Peisar } = require('peisar')

const document = new Peisar('# Title', {
  gfm: false,
  kramdown: false,
  fragment: true,
})

console.log(document.html) // <h1>Title</h1>
```

Supported options:

- `gfm` and `kramdown` enable their respective syntax extensions (default:
  `true`).
- `fileName` attaches a source name to the returned AST.
- `fragment` returns only rendered body content when `true` (default:
  `false`).
- `charset`, `viewport`, `title`, `bodyClass`, and `style` configure the full
  HTML document. `bodyClass` is applied to `<body>`.

## AST visitors

```js
const { Peisar } = require('peisar')

const document = new Peisar('# Hello', { fragment: true })

document.useVisitor({
  visitBlock([block]) {
    if (block.type === 'Heading') {
      return {
        replaceWith: [{ ...block, attrs: { ...block.attrs, classes: ['title'] } }],
      }
    }
    return { recurse: true }
  },
})

console.log(document.html) // <h1 class="title">Hello</h1>
```

## Custom parser hooks

Visitors transform the AST *after* parsing; parser hooks let extensions
parse custom syntax *during* parsing. Hooks run **before** the built-in
parsers (so they can both introduce new syntax and override built-ins), in
registration order — the first hook to claim a position wins.

`useParser` takes an object with two optional callbacks:

```js
const document = new Peisar(':::note\nmy note\n:::\n\nSee [[Some Page]].', {
  fragment: true,
})

// Positions must be complete objects — the hook's own `pos` value is a
// placeholder; accurate spans are computed automatically.
const zero = { line: 0, column: 0, offset: 0 }

document.useParser({
  // Block hook: receives { line, lineIndex, lines }
  parseBlock([{ line, lines }]) {
    if (!line.startsWith(':::')) return // decline — let other parsers try
    const name = line.slice(3).trim()
    const closeIndex = lines.findIndex((l, i) => i > 0 && l.trim() === ':::')
    if (closeIndex === -1) return
    return {
      block: {
        type: 'HtmlBlock',
        html: `<div class="note" data-name="${name}"></div>`,
        pos: { start: zero, end: zero },
      },
      consumed: closeIndex + 1, // lines consumed (at least 1)
    }
  },

  // Inline hook: receives { rest, index }
  parseInline([{ rest }]) {
    if (!rest.startsWith('[[')) return // decline
    const close = rest.indexOf(']]')
    if (close === -1) return
    const target = rest.slice(2, close)
    return {
      inline: {
        type: 'Link',
        text: [{ type: 'Text', value: target, pos: { start: zero, end: zero } }],
        url: `https://wiki.example.com/${target.replace(/ /g, '_')}`,
        autolink: false,
        pos: { start: zero, end: zero },
      },
      consumed: close + 4, // characters consumed
    }
  },
})

console.log(document.html)
// <div class="note" data-name="note"></div>
// <p>See <a href="https://wiki.example.com/Some_Page">Some Page</a>.</p>
```

Notes:

- Hooks apply everywhere Markdown is parsed — headings, emphasis/link
  children, block quotes, list items, and table cells included.
- `parseBlock` must consume **at least one line**; `parseInline` should
  report `consumed: 0` (or return `undefined`) to decline.
- Source spans of hook-returned nodes are computed from `consumed`; a
  Kramdown `{:...}` attribute line after a hook-parsed block is applied
  automatically.
- Because the document is parsed at construction, `useParser` re-parses the
  original Markdown. Register hooks before reading `ast`, `html`,
  `frontmatter`, or `astJson`.

## YAML front matter

Leading YAML front matter is excluded from the rendered Markdown and exposed
through `frontmatter`:

```js
const document = new Peisar(`---
title: Hello
---

# Hello`)

console.log(document.frontmatter) // { title: 'Hello' }
```

## Rust API

The same parser is available as a plain Rust library — no Node.js runtime
required. The crate builds both an `rlib` (for Rust consumers) and a
`cdylib` (the Node addon), so adding it as a dependency gives you the full
parsing API.

```toml
[dependencies]
peisar = { git = "https://github.com/phothinmg/peisar" }
```

*(Once published, replace this with `peisar = "0.1"`.)*

### Parse Markdown into an AST

`Document::parse` is the entry point. `AstOptions` controls the enabled
extensions, and every node carries a zero-based source `Span`:

```rust
use peisar::token::Block;
use peisar::{AstOptions, Document};

let doc = Document::parse("# Hello **world**\n", &AstOptions::default(), None);

assert_eq!(doc.node_type, "root");
match &doc.children[0] {
    Block::Heading { level, pos, .. } => {
        assert_eq!(*level, 1);
        assert_eq!((pos.start.line, pos.start.column), (0, 0));
    }
    _ => unreachable!(),
}
```

Strict CommonMark (disable GFM and Kramdown) with a file name attached:

```rust
use peisar::{AstOptions, Document};

let opts = AstOptions {
    gfm: false,
    kramdown: false,
    file_name: Some("doc.md".into()),
};

let doc = Document::parse("# Title\n", &opts, opts.file_name.clone());
assert_eq!(doc.file_name.as_deref(), Some("doc.md"));
```

### Parse and render HTML

`Peisar` mirrors the JavaScript class: parse once, then read `html`,
`ast`, `frontmatter`, and `ast_json`:

```rust
use peisar::{Peisar, PeisarOptions};

let mut document = Peisar::new("# Hello **world**".into(), None);

// Default options: GFM + Kramdown enabled, full HTML document.
println!("{}", document.html());
// <!DOCTYPE html>
// <html>
// <head>…</head>
// <body>
// <h1>Hello <strong>world</strong></h1>
// </body>
// </html>

// `fragment: true` renders just the body content.
let mut fragment = Peisar::new(
    "# Hello **world**".into(),
    Some(PeisarOptions {
        fragment: Some(true),
        ..Default::default()
    }),
);
println!("{}", fragment.html()); // <h1>Hello <strong>world</strong></h1>
```

`PeisarOptions` is the same shape as the JavaScript options object —
`gfm`, `kramdown`, `fileName`, `fragment`, `charset`, `viewport`, `title`,
`bodyClass`, and `style` — with every field optional.

### AST, front matter, and JSON

`PeisarAst` is the pure-Rust document holder used by `Peisar`. It parses
front matter eagerly and re-runs registered visitors before each read:

```rust
use peisar::token::Block;
use peisar::PeisarAst;

let mut ast = PeisarAst::new("---\ntitle: Hello\n---\n\n# Hello".into(), None);

// YAML front matter as a serde_json value.
assert_eq!(
    ast.get_frontmatter().unwrap()["title"].as_str(),
    Some("Hello")
);

// The parsed AST (front matter already stripped).
assert!(matches!(&ast.get_ast().children[0], Block::Heading { level: 1, .. }));

// Serialized AST JSON.
let json = ast.ast_json();
assert!(json.contains("\"type\":\"heading\""));
```

Front matter is also available standalone through the `frontmatter` module:

```rust
use peisar::frontmatter::parse_markdown_frontmatter;

let parsed = parse_markdown_frontmatter("---\ntitle: Hello\n---\n\n# Hello")?;

let md: &str = parsed.pure_markdown_content(); // "# Hello"
let yaml = parsed.yaml_data().unwrap();        // {"title": "Hello"}

// Or take both by value in one call.
let (md, yaml) = parsed.into_parts();
```

### AST visitors

Implement the `AstVisitor` trait and run it with `visit_document_mut` to
transform the AST before rendering. Each callback receives a `&mut` node and
returns a control struct that can keep, replace, remove, or insert around
it:

```rust
use peisar::token::{Block, Inline};
use peisar::visitor::{InlineVisitControl, VisitControl, visit_document_mut};
use peisar::{AstOptions, AstVisitor, Document};

struct AddTitleClass;

impl AstVisitor for AddTitleClass {
    fn visit_block(&mut self, block: &mut Block) -> VisitControl {
        if let Block::Heading { attrs: slot, .. } = block {
            let mut attrs = slot.take().unwrap_or_default();
            let mut classes = attrs.classes.take().unwrap_or_default();
            classes.push("title".into());
            attrs.classes = Some(classes);
            *slot = Some(attrs);
            VisitControl::keep_and_recurse() // mutate in place, then recurse
        } else {
            VisitControl::default() // leave everything else untouched
        }
    }

    fn visit_inline(&mut self, _inline: &mut Inline) -> InlineVisitControl {
        InlineVisitControl::default()
    }
}

let mut doc = Document::parse("# Hello\n", &AstOptions::default(), None);
visit_document_mut(&mut doc, &mut AddTitleClass);
```

`VisitControl` / `InlineVisitControl` support `insert_before`,
`insert_after`, `replace_with`, `remove`, and `recurse` (plus
`keep_and_recurse()` / `remove()` / `replace_with(nodes)` convenience
constructors). Replacements are not re-visited.

### Inline parsing

`parse_inline` parses inline content in isolation (for table cells,
headings, or plain text). `parse_inline_with_refs` additionally resolves
reference-style links against a `LinkRefMap`:

```rust
use peisar::token::Inline;
use peisar::{LinkRefMap, parse_inline, parse_inline_with_refs};

let inlines = parse_inline("*hi* and `code`", None);
assert!(matches!(&inlines[0], Inline::Emphasis { .. }));

// Resolve `[label][]` against collected link reference definitions.
let mut refs = LinkRefMap::new();
refs.insert("example".into(), ("https://example.com".into(), None));

let inlines = parse_inline_with_refs("see [example][]", None, Some(&refs));
assert!(matches!(
    &inlines[1],
    Inline::LinkReference { label, url, .. }
        if label == "example" && url == "https://example.com"
));
```

### Custom parser hooks (Rust)

The `AstParser` trait is the Rust counterpart of the JavaScript `useParser`
hooks — hooks run before the built-in matchers, so they can introduce new
syntax or override built-ins:

```rust
use peisar::token::Block;
use peisar::{AstParser, BlockParseContext};

struct Directive;

impl AstParser for Directive {
    fn try_parse_block(&self, ctx: &BlockParseContext) -> Option<(Block, usize)> {
        let name = ctx.line.trim().strip_prefix(":::")?;
        if name.is_empty() {
            return None; // decline — let other parsers try
        }
        let close = ctx.lines.iter().skip(1).position(|l| l.trim() == ":::")?;
        Some((
            Block::HtmlBlock {
                html: format!("<div class=\"directive\" data-name=\"{name}\">"),
                pos: Default::default(), // real span computed automatically
                attrs: None,
            },
            close + 2, // number of source lines consumed
        ))
    }
}
```

`ParseHooks::empty().with(&hook)` builds the ordered hook registry (the
first hook to claim a position wins). Note that hook-armed document parsing
is currently wired through the JS-facing registration APIs —
`PeisarAst::add_parser` / `Peisar::use_parser` — which wrap JS callbacks in
this same trait; pure-Rust hook registration for `Document::parse` is not
yet exposed at the crate root.

### Rust API surface

Everything above is re-exported from the crate root:

| Item | Kind | Purpose |
| --- | --- | --- |
| `Peisar` | struct | Parse + render in one place (mirrors the JS class) |
| `PeisarAst` | struct | Own the AST, visitors, hooks, front matter |
| `PeisarOptions` | struct | Combined parse + render options (JS shape) |
| `Document` | struct | AST root; `Document::parse` entry point |
| `AstOptions` | struct | `gfm`, `kramdown`, `file_name` |
| `parse_inline` / `parse_inline_with_refs` | fn | Standalone inline parsing |
| `LinkRefMap` | type | `label → (url, title)` map for reference links |
| `AstVisitor` | trait | Post-parse AST transforms |
| `AstParser` | trait | During-parse extension hooks |
| `BlockParseContext` / `InlineParseContext` | struct | Data given to hooks |
| `ParseHooks` | struct | Ordered hook registry |
| `token` | module | `Block`, `Inline`, `Table`, `ListItem`, `Attributes`, … |
| `span` | module | `Span`, `Position` source mapping |
| `visitor` | module | `VisitControl`, `InlineVisitControl`, `visit_document_mut` |
| `frontmatter` | module | `parse_markdown_frontmatter`, `ParseResult` |

Run the API docs locally with:

```sh
cargo doc --no-deps --open
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

```text
src/
├── ast/            AST definitions, parsers, visitors, and tests
├── config/         JavaScript option conversion
├── frontmatter.rs  YAML front-matter extraction
├── html.rs         HTML renderer
└── lib.rs          Public Rust API + Node.js N-API bindings
```

The generated TypeScript API is available in [index.d.ts](index.d.ts);
the Rust API is documented in the source and via `cargo doc`.
