<!-- markdownlint-disable MD033 -->
<!-- markdownlint-disable MD041 -->
<div align="center">
<img src="https://pub-c9ba018358dd48a99b70013b65a25e5f.r2.dev/logo/peisar.webp" width="160" height="160" alt="peisar" />
  <h1>Peisar</h1>
  <p>A Markdown parser and HTML renderer for Node.js, written in Rust.</p>
</div>

---

[![NPM](https://nodei.co/npm/peisar.svg)](https://nodei.co/npm/peisar/)

---


Peisar parses CommonMark Markdown with GitHub Flavored Markdown (GFM),
Kramdown-style block attributes, YAML front matter, source spans, AST
visitors, custom parser hooks, and configurable HTML output.

## Features

- **CommonMark** — headings, paragraphs, code blocks, block quotes, lists,
  thematic breaks, HTML blocks, emphasis, links, images, inline code, and
  hard or soft breaks.
- **GFM** — tables with column alignment, strikethrough, task lists, and
  autolinks.
- **Kramdown attributes** — `{:#id .class key="value"}` on block elements.
- **YAML front matter** — metadata is available separately from the parsed
  Markdown.
- **AST visitors** — transform the parsed AST from JavaScript before reading
  it or rendering HTML.
- **Custom parser hooks** — parse extension syntax (directives, wikilinks,
  mentions) during parsing, before the built-in matchers.
- **Node.js bindings** — native N-API addon with TypeScript declarations.

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
└── lib.rs          Node.js N-API bindings
```

The generated TypeScript API is available in [index.d.ts](index.d.ts).
