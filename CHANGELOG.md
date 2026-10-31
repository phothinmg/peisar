# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [unrelease]

## [1.2.0] - 2026-10-31

### Added

- Added theme-package loading to `PeisarCache`, including validated
  `theme.config.json` metadata and theme assets.
- Added default and custom layout filtering when discovering Markdown pages.
- Added `PeisarCache.usePlugin()` for applying visitors and parser hooks to
  cached page rendering.

### Changed

- Cache entry directories now default to the current working directory,
  require a supported root `index` Markdown file, and only cache Markdown
  pages with a recognized `layout` front-matter value.
- Assets are no longer implicitly loaded from `public`; configure an asset
  directory explicitly, and keep it within the entry directory.
- Cache configuration and theme-loading failures are surfaced as errors.
- Updated the README, API references, and Agent Skills to document the
  current cache, plugin, and release APIs.

## [1.1.0] - 2026-10-25

### Added

- Unified JavaScript plugin registration through `Peisar.usePlugin()`:
  a single plugin object can provide `visitBlock`, `visitInline`,
  `parseBlock`, and `parseInline` callbacks. Plugins run automatically
  before AST, HTML, front-matter, and JSON access; parser hooks re-parse the
  original Markdown and visitor changes are applied once per registration.
- Unified the Rust extension traits behind `PluginFactory`, so parser hooks
  and AST visitors share one registration and traversal pipeline.
- JavaScript integration coverage for visitor replacement, custom block and
  inline parser hooks, and their composition.

### Changed

- `useVisitor()` and `useParser()` remain available as deprecated,
  backward-compatible aliases for the visitor and parser portions of
  `usePlugin()`.
- Refreshed the Next.js and Vite examples to depend on Peisar 1.0.0.
- Removed locally generated N-API build artifacts from the repository; the
  local build script generates the CommonJS and ESM loaders when needed.

### Changed

- JavaScript plugin visitor callbacks now use and document their one-item
  tuple node argument (`visitBlock([block])`). Failures in `visitBlock` or
  `visitInline` are reported instead of being silently treated as no-ops.

## [1.0.0] - 2026-10-09

### Changed

- `PeisarEmphasisLevel`, `PeisarTaskState`, `PeisarTableCellAlignment`, and
  `PeisarTableCellAlignments` type unions in the generated TypeScript
  declarations (via the `dtsHeaderFile` in `napi.config.json`); in the
  JavaScript AST, `Emphasis.level`, `ListItem.task`, and
  `Table.alignments` now surface as string literals (`"Bold"`,
  `"Checked"`, `["Left", "Center"]`) instead of numeric enums
- N-API bindings are now optional behind the `npm` Cargo feature: the crate
  builds as a plain `rlib` without it, and `npm run build` / `npm run js:test`
  enable it explicitly
- Renamed the npm scripts to match their current behavior:
  `build:local` → `build` and `wf:test` → `js:test`; `js:test` builds a
  fresh addon in a temporary directory and runs the Node.js integration
  tests against it, covering both the build and the test step after
  N-API-visible Rust changes
- Bumped the README's Rust dependency snippet to the current release

### Notes

- Versions 0.3.1 through 0.3.13 were npm-only release iterations (npm's
  latest, 0.3.13, already ships these changes), while crates.io stayed at
  0.3.11. Version 1.0.0 realigns both registries and marks the public API
  stable: the AST, renderer, front matter, and N-API surfaces are
  covered by the test suites and are not expected to change within the
  1.x series.

## [0.3.0] - 2026-10-06

### Added

- WebAssembly (WASI) build, published as the
  `@peisar/peisar-wasm32-wasi` package:
  - A `wasm32-wasip1-threads` target in `napi.config.json`, built on
    CI and uploaded with the release artifacts (`.wasm`,
    `peisar.wasi.cjs`, `peisar.wasi.d.cts`, `peisar.wasi-browser.js`,
    and the WASI worker scripts)
  - `emnapi` dependencies (`emnapi`, `@emnapi/core`,
    `@emnapi/runtime`) for the WASM runtime
- Node.js `engines` requirement
  (`^20.19.0 || ^22.13.0 || >=23.5.0 || ^24.21.0`) in the root and
  platform packages
- A `license` field (`Apache-2.0`) in `package.json`

### Fixed

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
- Security policy (`SECURITY.md`): supported versions, the renderer's trust
  model, and a private vulnerability reporting process
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

[1.2.0]: https://github.com/phothinmg/peisar/compare/1.1.0...1.2.0
[1.0.0]: https://github.com/phothinmg/peisar/compare/0.3.0...1.0.0
[0.3.0]: https://github.com/phothinmg/peisar/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/phothinmg/peisar/compare/0.1.2...0.2.0
[0.1.2]: https://github.com/phothinmg/peisar/compare/v0.1.0...v0.1.2
[0.1.0]: https://github.com/phothinmg/peisar/releases/tag/v0.1.0