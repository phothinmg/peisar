# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Rustdoc for the `peisar::cache` module (module-level overview with a Rust
  usage example, discovery/disk-format/threading sections, and doc comments
  for `PeisarCache`, `CachedContent`, `start_watching`, `get`, `all`, and the
  file-discovery helpers in `cache::file`)
- A "File cache" subsection in the README's Rust section covering
  `PeisarCache::with_config`, `all()`, `get()`, `start_watching()`, and the
  drop-based cleanup, compile-checked by a new `cache_paths` test in
  `tests/readme_paths.rs`
- `PeisarEmphasisLevel`, `PeisarTaskState`, `PeisarTableCellAlignment`, and
  `PeisarTableCellAlignments` type unions in the generated TypeScript
  declarations (via the `dtsHeaderFile` in `napi.config.json`); in the
  JavaScript AST, `Emphasis.level`, `ListItem.task`, and
  `Table.alignments` now surface as string literals (`"Bold"`,
  `"Checked"`, `["Left", "Center"]`) instead of numeric enums

### Changed

- N-API bindings are now optional behind the `npm` Cargo feature: the crate
  builds as a plain `rlib` without it, and `npm run build:local` (and the
  `napi` invocations in `scripts/build.sh` / `scripts/wf_tests.sh`) enable
  it explicitly
- Bumped the README's Rust dependency snippet to the published `0.3.0`
  version

## [0.3.0] - 2026-10-06

### Added

- WebAssembly (WASI) build, published as the
  `@peisar/peisar-wasm32-wasi` package:
  - A `wasm32-wasip1-threads` target in `napi.config.json`, built on
    CI and uploaded with the release artifacts (`.wasm`,
    `peisar.wasi.cjs`, `peisar.wasi.d.cts`, `peisar.wasi-browser.js`,
    and the WASI worker scripts)
  - A `browser` entry (`browser.js`) in the root package, so bundlers
    resolve the WASM build instead of the native addon
  - `emnapi` dependencies (`emnapi`, `@emnapi/core`,
    `@emnapi/runtime`) for the WASM runtime
- Node.js `engines` requirement
  (`^20.19.0 || ^22.13.0 || >=23.5.0 || ^24.21.0`) in the root and
  platform packages
- A `license` field (`Apache-2.0`) in `package.json`
- `PeisarCache` Node.js export: the in-memory Markdown/asset file cache with
  `.peisar_cache` disk persistence and recursive file-watching, restored for
  the `peisar-ssg` package:
  - `new PeisarCache(entryDir, assetsDir?)` constructor and a
    `PeisarCache.withConfigJs` factory
  - `startWatchingJs()` to start the recursive watcher
  - `getText(absPath)` / `getBinary(absPath)` cached-content accessors
  - `listFiles()`, `markdownFiles()`, and `assetFiles()` path listings
  - `onChange(cb) → id` / `offChange(id)` change subscriptions delivering
    `{ path, kind, isMarkdown }` events
  - `dispose()` to stop the watcher and release the persistence worker so the
    Node process can exit
- A `cache` module (`peisar::cache`) in the Rust crate powering the above,
  along with the `blake3`, `notify`, and `bincode` dependencies it needs

### Fixed

- Cache targets for paths outside the working directory no longer clobber the
  original source file: `PathBuf::join` with an absolute path replaces the
  buffer, so paths are now mirrored by their normal components
  (e.g. `/elsewhere/b.png` → `.peisar_cache/elsewhere/b.png`)
- Watcher events now carry `isMarkdown: true` correctly when `PeisarCache`
  was constructed with a relative entry directory (notify delivers absolute
  paths, so the entry dir is normalized to absolute at construction)

### Changed

- Updated the README: a new tagline ("A practical Markdown parser
  written in Rust"), a Next.js integration section documenting
  `serverExternalPackages`, and removal of the `peisar-ssg` migration
  notes

### Notes

- Versions 0.2.1 through 0.2.22 were npm-only release iterations
  (npm's latest, 0.2.22, already ships these changes), while
  crates.io stayed at 0.2.0. Version 0.3.0 realigns both registries.
- The new `engines` requirement drops Node.js runtimes older than
  20.19 — a breaking change for consumers on older runtimes; Semantic
  Versioning permits breaking changes between 0.x minor versions.
- `@peisar/peisar-wasm32-wasi` is intentionally not part of the root
  package's `optionalDependencies` (the @napi-rs/cli default), so npm
  never downloads the `.wasm` binary for consumers whose native binary
  loads. WebAssembly-restricted environments install it explicitly:
  `npm i peisar @peisar/peisar-wasm32-wasi`. The README documents the
  `NAPI_RS_FORCE_WASI` loader variables.


## [0.2.0] - 2026-09-30

### Added

- Custom parser hooks via `useParser`: block hooks (`parseBlock`) and inline
  hooks (`parseInline`) run before built-in matchers, in registration order,
  with consume-or-decline semantics
- High-level `peisar::markdown::peisar::Peisar` parse-and-render wrapper for
  Rust consumers
- Git hooks: a `commit-msg` hook enforcing the commit message convention
  (`.githooks/commit-msg`), plus `scripts/commit.sh` (`npm run commit`) and
  `scripts/install-hooks.sh` (`npm run hooks:install`)
- Agent Skills for AI coding tools, published with
  `npx skills add phothinmg/peisar`:
  `peisar-parse-markdown`, `peisar-ast-visitors`, `peisar-parser-hooks`, and
  `peisar-development`
- Integration tests that compile-check every documented Rust and Node.js API
  path in `README.md` (`tests/readme_paths.rs`)
- Security policy (`SECURITY.md`): supported versions, the removal notice for
  the `PeisarCache` components that moved to `peisar-ssg`, the renderer's
  trust model, and a private vulnerability reporting process
- CI workflow that tests on Ubuntu, macOS, and Windows, and builds the native
  addon for eight targets (GNU/MUSL Linux x64/arm64, macOS x64/arm64, and
  Windows x64/arm64) on every published release

### Changed

- Restructured the crate into a module-based layout; Rust consumers now use
  module paths instead of root re-exports:
  - `peisar::markdown::ast` — parser, AST nodes, options, visitors, hooks
  - `peisar::markdown::html` — HTML renderer and `RenderOptions`
  - `peisar::markdown::config` — combined `PeisarOptions`
  - `peisar::markdown::peisar` — high-level document wrapper
  - `peisar::frontmatter` — YAML front-matter parser
- Expanded the parser test suite (`src/markdown/ast/tests.rs`) and the
  Node.js integration tests (`js_tests/peisar.test.txt`) to cover front
  matter, visitors, hooks, and their composition
- Rewrote documentation: README, CONTRIBUTING, and the Agent Skills

### Removed

- Static-site support, moved to the separate `peisar-ssg` project:
  - The `PeisarCache` and `peisarSsgConfig()` Node.js exports
  - The `peisar::ssg::{cache, files, ssg_config}` Rust modules
  - `Peisar.toml` configuration loading and the `schema/peisar-ssg.json`
    JSON Schema
- The `blake3`, `notify`, `bincode`, and `toml` dependencies, which served
  only the removed cache and configuration loader

### Notes

- Versions 0.1.3, 0.1.4, and 0.1.5 were published to npm only, while
  crates.io stayed at 0.1.2. Version 0.2.0 realigns both registries and
  documents all changes shipped in those interim releases.
- Removing public APIs is a breaking change for Rust and Node.js consumers;
  Semantic Versioning permits breaking changes between 0.x minor versions.

## [0.1.2] - 2026-09-28

### Fixed

- Package metadata corrections for the npm release

## [0.1.0] - 2026-09-28

### Added

- Initial public release of Peisar, a Markdown parser and HTML renderer
  written in Rust with native Node.js bindings
- CommonMark block and inline parsing, including raw HTML and reference links
- GitHub Flavored Markdown: tables, strikethrough, task lists, and autolinks
- Kramdown block attributes such as `{:#intro .lead key="value"}`
- YAML front matter, returned separately from the rendered Markdown
- Source spans on every AST node
- AST visitors (`useVisitor`) with control objects supporting `recurse`,
  `insertBefore`, `insertAfter`, `replaceWith`, and `remove`
- Native N-API addon with generated TypeScript declarations, exporting the
  `Peisar` class with `ast`, `html`, `frontmatter`, `astJson`, `useVisitor`,
  and parse/render options (GFM and Kramdown enabled by default)
- Rust `rlib` crate and Node.js integration test suite

[Unreleased]: https://github.com/phothinmg/peisar/compare/0.3.0...HEAD
[0.3.0]: https://github.com/phothinmg/peisar/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/phothinmg/peisar/compare/0.1.2...0.2.0
[0.1.2]: https://github.com/phothinmg/peisar/compare/v0.1.0...v0.1.2
[0.1.0]: https://github.com/phothinmg/peisar/releases/tag/v0.1.0