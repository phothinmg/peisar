---
name: peisar-development
description: Build, test, and modify Peisar's Rust implementation and native Node.js bindings. Use when changing Markdown parsing, HTML rendering, front matter, YAML handling, or N-API surfaces.
---

# Peisar Development

Peisar is a Rust crate that builds both an `rlib` and an N-API `cdylib`.
The crate provides a high-level parser wrapper and delegates the lower-level
AST, HTML, plugin, and YAML implementations to `peisar_ast` and `peisar_yaml`.

## Layout

```text
src/
├── ast.rs                             feature-gated AST re-exports
├── core/
│   ├── config.rs                      parser and rendering options
│   ├── html.rs                        HTML rendering integration
│   └── parser.rs                      Peisar wrapper and N-API binding
├── html.rs                            feature-gated renderer re-exports
├── plugin.rs                          feature-gated plugin re-exports
├── yaml.rs                            feature-gated YAML re-exports
└── lib.rs                             public modules and root re-exports
```

`Peisar`, `PeisarOptions`, and `get_options` are re-exported at the crate root.
The lower-level Rust modules require the matching `ast`, `html`, `plugin`, or
`yaml` Cargo feature. The Node.js `Peisar` class exposes `ast`, `html`,
`frontmatter`, `astJson`, and `use()`. The generated TypeScript declarations
are output by the N-API build; do not hand-edit them.

## Build and test

```sh
npm install
npm run js:test
npm run build
cargo test
```

`npm run js:test` builds a fresh addon in a temporary directory, regenerates
the package loader and TypeScript declarations, and runs the Node integration
tests in `scripts/peisar.test.txt`. `npm run build` produces the published
package files (`index.cjs`, `index.mjs`, `index.d.ts`). Use `cargo test` for
Rust tests.

## Change guidelines

- AST, renderer, plugin, and YAML implementation changes belong in their
  respective dependency crates; update this crate's re-exports or N-API
  wrapper only when its integration surface changes.
- Prefer `Peisar.use()` for Node.js plugin integrations.
- napi numerics: config fields crossing the boundary use `i64`/`u32` (not
  `u64`/`usize`); clamp in the resolved accessors.
- Rust consumers use `peisar::ast`, `peisar::html`, `peisar::plugin`, and
  `peisar::yaml` when enabling those features. `Peisar` and its options are
  available from the crate root.
