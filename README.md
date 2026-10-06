<!-- markdownlint-disable MD033 -->
<!-- markdownlint-disable MD041 -->
<div align="center">
<img src="https://pub-c9ba018358dd48a99b70013b65a25e5f.r2.dev/logo/peisar.webp" width="160" height="160" alt="Peisar logo" />
  <h1>Peisar</h1>
  <p>A practical Markdown parser written in Rust</p>
</div>

---

[![NPM](https://nodei.co/npm/peisar.svg)](https://nodei.co/npm/peisar/)

[![Documentation][docs_img]][docs]

**The project is still under active development, and its API may change. It will stabilize in the next major release.**

Peisar parses CommonMark Markdown with GitHub Flavored Markdown (GFM),
Kramdown block attributes, YAML front matter, source spans, AST visitors, and
custom parser hooks.

## Features

- CommonMark blocks and inlines, including HTML and reference links
- GFM tables, strikethrough, task lists, and autolinks
- Kramdown block attributes such as `{:#intro .lead key="value"}`
- YAML front matter, returned separately from rendered Markdown
- Source spans on every AST node
- AST visitors and during-parse extension hooks
- A native N-API Node.js addon with generated TypeScript declarations
- A Rust library (`rlib`) with public module paths for the parser, renderer,
  and high-level document APIs

## Node.js

Install the package:

```sh
npm i peisar
```

### Next.js 

**Example App :** <https://github.com/phothinmg/peisar/tree/main/examples/next>

**Demo Web APP :** <https://peisar-next-example.vercel.app/>

Next.js provides a built-in configuration option called `serverExternalPackages`.This opts specific dependencies out of Server Component bundling and allows the server to use a native Node.js `require` to load them directly from `node_modules`.

Add `peisar` to `serverExternalPackages` in your configuration file:

For `next.config.ts` (TypeScript)

```ts
import type { NextConfig } from 'next'

const nextConfig: NextConfig = {
  serverExternalPackages: ["peisar"],
}

export default nextConfig

```

For `next.config.js` or `next.config.mjs` (JavaScript)

```js
/** @type {import('next').NextConfig} */
const nextConfig = {
  serverExternalPackages: ["peisar"],
}

module.exports = nextConfig // or export default nextConfig if using .mjs

```

### Vite

**Example App :** <https://github.com/phothinmg/peisar/tree/main/examples/vite>

**Demo Web APP :** <https://peisar-vite-example.vercel.app/>

Vite browser applications must use Peisar's WebAssembly package rather than
the native `peisar` package:

```sh
npm i @peisar/peisar-wasm32-wasi
```

Import it directly in client-side code:

```ts
import { Peisar } from "@peisar/peisar-wasm32-wasi";

const document = new Peisar("# Hello from Vite", { fragment: true });
console.log(document.html);
```

The WebAssembly package uses threads and requires cross-origin isolation. Add
the required headers and exclude the package from Vite's dependency optimizer
so its internal worker is handled correctly:

```ts
// vite.config.ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  server: {
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
  plugins: [
    react(),
    {
      name: "configure-preview-response-headers",
      configurePreviewServer(server) {
        server.middlewares.use((_req, res, next) => {
          res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
          res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
          next();
        });
      },
    },
  ],
  optimizeDeps: {
    exclude: ["@peisar/peisar-wasm32-wasi"],
  },
});
```

After changing this configuration, clear Vite's optimized-dependency cache and
restart the development server:

```sh
rm -rf node_modules/.vite
npm run dev
```

For production builds, configure the same headers in the server or hosting
platform that serves the app. Vite's `server` and `preview` headers apply only
to local development and `vite preview`. Verify the deployed app reports
`globalThis.crossOriginIsolated === true`. With
`Cross-Origin-Embedder-Policy: require-corp`, cross-origin assets must also
allow embedding through CORS or `Cross-Origin-Resource-Policy`.

### WebAssembly (WASM) environments

The main package loads a native binary matching your platform, selected by
npm through `optionalDependencies`. Hosts that cannot load native `.node`
binaries — WebAssembly-restricted environments such as some edge runtimes —
fall back to the WebAssembly build, `@peisar/peisar-wasm32-wasi`.

That package is deliberately **not** installed by default: npm would download
the `.wasm` binary for every consumer, including the ones whose native
binary loads. Install it explicitly when you need the WebAssembly build:

```sh
npm i peisar @peisar/peisar-wasm32-wasi
```

The generated loader resolves the WebAssembly binding automatically whenever
no native binary can be loaded. To control the preference, set an environment
variable:

- `NAPI_RS_FORCE_WASI=true` — prefer the WebAssembly build and keep the native
  binding as a lazy fallback
- `NAPI_RS_FORCE_WASI=error` — require the WebAssembly build and fail when it
  is missing

In browser bundles, the root package's `browser.js` entry re-exports
`@peisar/peisar-wasm32-wasi`, so bundlers pick up the WebAssembly build once
the package is installed.

### Use

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

`ast` and `astJson` serialize the same tree differently: `ast` (the real
JavaScript object) uses PascalCase `type` values (`Heading`, `Paragraph`,
`Link`) and a `nodeType: "root"` root, while `astJson` mirrors the Rust
serialization with snake_case types (`heading`, `paragraph`, `link`) and a
`type: "root"` root.

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
An omitted return value keeps the node without recursing.

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

### File cache

`PeisarCache` collects every Markdown file under an entry directory (plus
optional assets) into memory, mirrors them to a `.peisar_cache` directory on
disk, and keeps both in sync with recursive file-watching once
`startWatchingJs()` is called. This is the engine behind
[`peisar-ssg`](https://github.com/phothinmg/peisar)'s dev server and build
pipeline.

```js
const { PeisarCache } = require("peisar");

// new PeisarCache(entryDir, assetsDir?)
const cache = new PeisarCache("contents", "public");

cache.markdownFiles(); // absolute paths of cached markdown files
cache.assetFiles(); // absolute paths of cached assets
cache.listFiles(); // everything cached

const raw = cache.getText(cache.markdownFiles()[0]); // cached file text, or null
const bytes = cache.getBinary(cache.assetFiles()[0]); // cached bytes, or null

cache.startWatchingJs(); // keep the cache in sync with the filesystem
const id = cache.onChange((event) => {
  // event: { path, kind: "create" | "modify" | "remove" | "other", isMarkdown }
});
cache.offChange(id); // unsubscribe
cache.dispose(); // stop the watcher and release worker threads
```

Paths are absolute on both sides: `markdownFiles()` returns absolute paths,
so consumers pass them straight back to `getText()` / `getBinary()`. Always
call `dispose()` when done so the watcher and persistence threads do not keep
the Node process alive.



## Rust

Add the crate to a Rust project:

```toml
[dependencies]
peisar = "0.3.0"
```

The public Rust API is module-based. Import the parser from
`peisar::markdown::ast`, the renderer from `peisar::markdown::html`, the
high-level document wrapper from `peisar::markdown::peisar`, and the file
cache from `peisar::cache`.

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
use peisar::frontmatter::frontmatter;

let parsed = frontmatter("---\ntitle: Hello\n---\n\n# Hello")?;
assert_eq!(parsed.yaml_data().unwrap()["title"], "Hello");
assert_eq!(parsed.pure_markdown_content(), "# Hello");
```

`parse_markdown_frontmatter` returns a `ParseResult` with `yaml_data()`,
`pure_markdown_content()`, and `into_parts()`. A front-matter block must open
with `---` on the first non-whitespace line and close with a line starting
`---`; an unterminated block is treated as ordinary Markdown body.

For post-parse traversal, implement
`peisar::markdown::ast::AstVisitor` and call
`peisar::markdown::ast::visitor::visit_document_mut`. The visitor controls
are `VisitControl` and `InlineVisitControl` from that same `visitor` module.

`AstParser` and `ParseHooks` are publicly exported from
`peisar::markdown::ast` for integration with Peisar internals, but the
hook-enabled parser entry point is not yet public to external Rust consumers.
Use `useParser` from Node.js for registered custom syntax today.

### File cache

The JavaScript `PeisarCache` class has a Rust equivalent,
`peisar::cache::PeisarCache`. It loads every Markdown file under an entry
directory (plus optional assets) into memory, mirrors them to a
`.peisar_cache` directory on disk, and keeps both in sync with recursive
file-watching once `start_watching()` is called.

```rust
use peisar::cache::{CachedContent, PeisarCache};

// Markdown from "contents", assets from "public". Relative paths are
// resolved against the current working directory.
let mut cache = PeisarCache::with_config("contents", Some("public"))?;

// Every cached Markdown file: absolute path -> raw text.
for (path, text) in cache.all() {
    println!("{} ({} bytes)", path.display(), text.len());
}

// Single entries keep their variant: Markdown is `Text`, binary assets
// are `Binary`.
let index = std::env::current_dir()?.join("contents").join("index.md");
match cache.get(&index) {
    Some(CachedContent::Text(md)) => println!("{md}"),
    Some(CachedContent::Binary(bytes)) => println!("{} bytes", bytes.len()),
    None => println!("not cached"),
}

// Keep memory and disk in sync with the filesystem.
cache.start_watching()?;
// Dropping the cache stops the watcher and joins the persistence
// worker; Rust consumers need no explicit dispose.
```

The cache is keyed by absolute paths, so keys from `all()` can be passed
straight back to `get()`. `all()` returns only text entries; use `get()` to
read binary assets. Construction persists the snapshot to `.peisar_cache`
and appends that directory to `.gitignore` when one exists; set
`PEISAR_CACHE_FORMAT=bincode` for compact `.bin` entries instead of the
default `.json`.


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

More detail in [CONTRIBUTING](CONTRIBUTING.md)

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
├── cache/                   Markdown/asset file cache (`PeisarCache`)
├── markdown/
│   ├── ast/                 AST nodes, parsers, visitors, and hooks
│   ├── config/              combined parser and renderer options
│   ├── html/                HTML renderer
│   └── peisar/              high-level N-API `Peisar` class
└── lib.rs                   public module declarations
```

## LICENSE
[Apache-2.0][license] © [Pho Thin Maung][ptm]

Security policy: [SECURITY.md](SECURITY.md)

<!-- Links and Badges -->
[license]: LICENSE
[ptm]: https://github.com/phothinmg
[docs]: https://docs.rs/peisar "Documentation"
[docs_img]: https://docs.rs/peisar/badge.svg "Documentation"
