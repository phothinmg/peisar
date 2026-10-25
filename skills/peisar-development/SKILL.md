---
name: peisar-development
description: Build, test, and modify Peisar's Rust implementation and native Node.js bindings. Use when changing Markdown parsing, HTML rendering, front matter, YAML handling, or N-API surfaces.
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
│   ├── mod.rs                          PeisarCache, watcher, archive coordination
│   ├── lru.rs                          bounded in-memory cache tier
│   ├── archive.rs                      disk shards and map.json index
│   └── obj_cache.rs                    page/asset objects and derived values
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
`useVisitor`, and `useParser`. The generated TypeScript declarations are
output by the N-API build; do not hand-edit them.

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
- napi numerics: config fields crossing the boundary use `i64`/`u32` (not
  `u64`/`usize`); clamp in the resolved accessors.
- Rust consumers use module paths such as `peisar::markdown::ast::Document`,
  not root re-exports.
- `PeisarCache` is exposed from `peisar::cache`; its cache implementation,
  watcher, and archive tests are colocated in `src/cache/`.
