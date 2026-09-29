---
name: peisar-development
description: Build, test, and modify Peisar's Rust implementation and native Node.js bindings. Use when changing Markdown parsing, HTML rendering, SSG configuration, cache behavior, or N-API surfaces.
---

# Peisar Development

Peisar is a Rust crate that builds both an `rlib` and an N-API `cdylib`.
Node.js bindings live beside the Rust implementation rather than in a
separate JavaScript source tree.

## Layout

```text
src/
├── frontmatter/mod.rs                 YAML front-matter extraction
├── markdown/
│   ├── ast/
│   │   ├── tokens/                    AST nodes, spans, and attributes
│   │   ├── parsers/                   block, inline, hooks, visitor traversal
│   │   ├── js.rs                      N-API callback adapters
│   │   └── mod.rs                     PeisarAst holder
│   ├── config/mod.rs                  PeisarOptions resolution
│   ├── html/mod.rs                    AST-to-HTML renderer
│   └── peisar/mod.rs                  exported N-API Peisar class
├── ssg/
│   ├── cache/mod.rs                   file cache and watcher
│   ├── files/mod.rs                   markdown and asset file discovery
│   └── ssg_config/mod.rs              Peisar.toml parser and loader
└── lib.rs                             public module declarations
```

The Node.js `Peisar` class exposes `ast`, `html`, `frontmatter`, `astJson`,
`useVisitor`, and `useParser`. It also exports `PeisarCache` and
`peisarSsgConfig`. The generated TypeScript declarations are output by the
N-API build; do not hand-edit them.

The `PeisarCache` JS surface is: `new PeisarCache(entryDir, assetsDir?)`,
`PeisarCache.withConfigJs(entryDir, assetsDir?)`, `getText(abs)`,
`getBinary(abs)`, `listFiles()`, `markdownFiles()`, `assetFiles()`,
`onChange(cb) → id`, `offChange(id)`, `startWatchingJs()`, and `dispose()`.
The cache keys by absolute path and mirrors its entries to `.peisar_cache`
on disk (JSON by default, `PEISAR_CACHE_FORMAT=bincode` for compact `.bin`
entries); `dispose()` must be called to release the watcher and worker
threads so Node can exit.

## Build and test

```sh
npm install
npm run build:local
npm test
cargo test
```

`npm run build:local` generates the package loader and declarations in the
local build output, then compiles the native addon. Run it before `npm test`
after changing N-API-visible Rust types or methods. Node integration tests are
in `test/peisar.test.js`; Rust parser tests are in
`src/markdown/ast/tests.rs`; documentation API paths are compile-checked in
`tests/readme_paths.rs`.

## Change guidelines

- Change AST shapes consistently in `tokens/token.rs`, N-API types in
  `ast/js.rs`, renderer handling in `html/mod.rs`, and tests.
- Parser hooks are defined in `ast/parsers/hooks.rs`; both block and inline
  hooks must preserve their consume-or-decline semantics.
- Visitor traversal is in `ast/parsers/visitor.rs`. Replacements intentionally
  are not traversed again.
- `PeisarAst` owns raw Markdown, front matter, hooks, and visitors. Adding a
  parser hook reparses the raw Markdown; document reads apply registered
  visitors.
- Rust consumers use module paths such as `peisar::markdown::ast::Document`,
  not root re-exports.
- The JavaScript-facing SSG loader exits on configuration failures; Rust
  consumers should use the fallible `load_config`.
