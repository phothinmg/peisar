---
name: peisar-development
description: Build, test, and modify Peisar's Rust implementation and native Node.js bindings. Use when changing Markdown parsing, HTML rendering or N-API surfaces.
---

# Peisar Development

Peisar is a Rust crate that builds both an `rlib` and an N-API `cdylib`.
Node.js bindings live beside the Rust implementation rather than in a
separate JavaScript source tree.

## Layout

```text
src/
├── frontmatter/mod.rs                 YAML front-matter extraction
├── cache/
│   ├── mod.rs                         PeisarCache: memory-first N-API class,
│   │                                  watcher, page/asset objects
│   ├── lru.rs                        size-aware LRU memory tier
│   ├── archive.rs                    binary shard archive + map.json worker
│   ├── base_url.rs                   hosting baseUrl resolution (GH Pages/…)
│   ├── obj_cache.rs                   PageObject/AssetsObject + ObjectCache
│   └── file.rs                       markdown/asset file discovery and extensions
├── markdown/
│   ├── ast/
│   │   ├── tokens/                    AST nodes, spans, and attributes
│   │   ├── parsers/                   block, inline, hooks, visitor traversal
│   │   ├── js.rs                      N-API callback adapters
│   │   └── mod.rs                     PeisarAst holder
│   ├── config/mod.rs                  PeisarOptions resolution
│   ├── html/mod.rs                    AST-to-HTML renderer
│   └── peisar/mod.rs                  exported N-API Peisar class
└── lib.rs                             public module declarations
```

The Node.js `Peisar` class exposes `ast`, `html`, `frontmatter`, `astJson`,
`useVisitor`, and `useParser`. The Node.js `PeisarCache` class is
memory-first: an LRU in-memory tier backed by a background archive worker
that flushes binary shards to `.peisar-cache` with a `map.json` search
index. It exposes `markdownFiles`, `assetFiles`, `listFiles`, `getText`,
`getBinary`, `pages`, `getPage`, `assets`, `getAsset`, `archived`, `mapJson`,
`flush`, `baseUrl`, `resolveUrl`, `siteUrl`, `useVisitor`, `useParser`,
`startWatchingJs`, `onChange`, `offChange`, and `dispose` (see
`src/cache/mod.rs`). The generated TypeScript declarations are output by
the N-API build; do not hand-edit them.

## Build and test

```sh
npm install
npm run js:test
npm run build
cargo test
```

`npm run js:test` builds a fresh addon in a temporary directory, regenerates
the package loader and TypeScript declarations, and runs the Node integration
tests in `scripts/peisar.test.txt` — it covers both the build and the test
step after changing N-API-visible Rust types or methods. `npm run build`
produces the published package files (`index.cjs`, `index.mjs`,
`index.d.ts`). Rust parser tests are in
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
- `PeisarCache` is memory-first: `LruCache` (src/cache/lru.rs) bounds the hot
  set by bytes/entries; evicted entries stay readable through the archive
  tier. The archive worker (src/cache/archive.rs) buffers upserts and writes
  bincode shards + `map.json` on size/interval/drop; shards are immutable,
  GC'd when unreferenced, and written atomically (tmp + rename).
- napi numerics: config fields crossing the boundary use `i64`/`u32` (not
  `u64`/`usize`); clamp in the resolved accessors.
- Rust consumers use module paths such as `peisar::markdown::ast::Document`,
  not root re-exports.
