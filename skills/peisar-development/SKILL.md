---
name: peisar-development
description: Build, test, and modify the Peisar Rust/Node.js Markdown parser codebase itself. Use when contributing to the peisar crate, debugging the parser, changing parsing behavior, or adding new Markdown features.
---

# Peisar Development

Guidance for working on the Peisar codebase — a Markdown parser and HTML renderer written in Rust, exposed to Node.js via N-API (napi-rs) and usable as a plain Rust library.

## When to Use

- Modifying any file under `src/` (the Rust parser, renderer, or napi bindings)
- Fixing a parsing bug or adding new Markdown syntax
- Debugging failed tests in `test/peisar.test.js` or `src/ast/tests.rs`
- Building the native addon (`peisar.linux-x64-gnu.node`)
- Publishing the package (`@peisar/peisar` on npm)

## Project Layout

```
src/
├── lib.rs              # napi class `Peisar` — the JS entry point
├── html
|   └── mod.rs          # AST → HTML renderer (`AstToHtml`, `RenderOptions`)
├── frontmatter
|   └── mod.rs          # YAML front matter extraction
├── config/
│   └── mod.rs          # `PeisarOptions` (JS-facing), option resolution
└── ast/
    ├── mod.rs          # `PeisarAst` — document holder, re-parse on hook registration
    ├── js.rs           # napi adapter layer (JS callbacks → Rust traits)
    ├── options.rs      # `AstOptions` (Rust-facing parse options)
    ├── tests.rs        # Rust unit tests
    ├── parsers/
    │   ├── mod.rs      # `Document` root type, `md_to_ast` entry point
    │   ├── block.rs    # block-level parser (headings, lists, quotes, code, HTML blocks)
    │   ├── inline.rs   # inline parser (emphasis, links, code, autolinks)
    │   ├── table.rs    # GFM table helpers
    │   ├── atters.rs   # Kramdown `{:#id .class}` attribute parsing
    │   ├── hooks.rs    # `AstParser` trait, `BlockParseContext`, `InlineParseContext`
    │   └── visitor.rs  # `AstVisitor` trait, `visit_document_mut`, `VisitControl`
    └── tokens/
        ├── token.rs    # `Block` and `Inline` enums (the AST node types)
        ├── span.rs     # `Span` / `Position` source tracking
        └── attrs.rs    # `Attributes` (id, classes, key/values)
```

Two parallel APIs exist:

- **Rust API** — `AstVisitor` trait (in `parsers/visitor.rs`), `AstParser` trait (in `parsers/hooks.rs`)
- **JavaScript API** — callback objects registered via `useVisitor` / `useParser`; `js.rs` adapts them to the Rust traits

When changing AST node shapes, update all three touchpoints: `tokens/token.rs` (definition), `js.rs` (napi mirrors), and `index.d.ts` (regenerated declarations).

## Build & Test Commands

```sh
npm install                    # install devDependencies (@napi-rs/cli)
npm run build:local            # napi build → rebuilds index.js/index.d.ts + .node addon
npm test                       # node --test (runs test/peisar.test.js)
cargo test                     # Rust unit tests (src/ast/tests.rs + doctests)
```

`npm run build:local` runs `npx napi build --platform --config-path napi.config.json` (see `scripts/build.sh`). It regenerates `index.js` and `index.d.ts` — do not hand-edit those files. After changing any napi-annotated Rust, rebuild before running `npm test`.

`cargo test` requires a nightly toolchain for some doctests (see README); run `npm test` after `npm run build:local` for the integration suite.

## Key Architecture Facts

### Parsing is eager and re-parseable

`PeisarAst::new` parses the Markdown at construction. Because hooks must run _during_ parsing, calling `add_parser` / `useParser` triggers `reparse()` of the stored `raw_md`. Visitors run before each read of `ast`, `html`, `astJson`, and `frontmatter`.

Consequence for JS users: **register hooks/visitors before reading any property.**

### Span computation

Every node carries `pos: Span` with zero-based `start`/`end` positions (`{ line, column, offset }`). The block parser tracks `pos` (line index) and computes spans via `self.span(start_line, self.pos)`. Hook-returned nodes may pass a placeholder span (`{ line: 0, column: 0, offset: 0 }` in JS or `Default::default()` in Rust); the engine overwrites it based on the hook's `consumed` count.

### Serialization shape

`Block`/`Inline` enums use `#[serde(tag = "type", rename_all = "snake_case")]`, so JSON node types are lowercase snake_case (`heading`, `paragraph`, `html_block`, `link_reference`). But the napi JS bindings serialize the **Rust enum variant name** (`Heading`, `Paragraph`, `HtmlBlock`, `LinkReference`). Both forms exist: `ast`/`astJson` from JS return PascalCase; pure `ast_json()` from Rust returns snake_case. Check which API surface you are testing against.

### Crate types

`Cargo.toml` builds both `rlib` (Rust consumers) and `cdylib` (Node addon). The crate name is `peisar`; lib name `peisar`. Platform binaries ship via `npm/<platform>/` optional dependencies (see `napi.config.json` targets).

## Debugging Workflow

1. Reproduce with a minimal Markdown snippet
2. Write a failing test in `test/peisar.test.js` (JS) or `src/ast/tests.rs` (Rust)
3. Fix in the relevant parser (`block.rs` for block-level, `inline.rs` for inline)
4. `npm run build:local && npm test` (JS path) or `cargo test` (Rust path)
5. Verify spans (`pos`) on affected nodes — many bugs are off-by-one span errors

## Testing Conventions

- JS tests live in `test/peisar.test.js` using `node:test` + `node:assert/strict`, loaded via `require("../index.js")`
- Rust tests live in `src/ast/tests.rs` (`#[cfg(test)]`)
- Use `{ fragment: true }` in JS tests to compare exact body HTML without the document wrapper
- Most rendered fragments end with `\n` — include it in assertions

## Common Pitfalls

- **Hand-editing `index.js` / `index.d.ts`** — they are generated by `napi build`. Change the Rust source instead.
- **Forgetting to rebuild** after napi changes — `npm test` loads the stale `.node` binary otherwise.
- **Hook `consumed` semantics** — block hooks must consume ≥ 1 line (`0` is treated as `1`); inline hooks returning `consumed: 0` are ignored.
- **Front matter parsing** — `parse_markdown_frontmatter` looks for `---\n` at the very start and the first `\n---`; a YAML error fails the whole parse.
- **Visitor replacements are not re-visited** — replacement nodes are inserted without recursing into them (prevents infinite loops).
