# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Custom parser hooks via `useParser`: block hooks (`parseBlock`) and inline
  hooks (`parseInline`) run before built-in matchers, in registration order,
  with consume-or-decline semantics
- High-level `peisar::markdown::peisar::Peisar` parse-and-render wrapper for
  Rust consumers
- `PeisarCache` Node.js and Rust API: `new PeisarCache(entryDir, assetsDir?)`,
  `PeisarCache.withConfigJs`, `getText`, `getBinary`, `listFiles`,
  `markdownFiles`, `assetFiles`, `onChange`/`offChange`, `startWatchingJs`,
  and `dispose`
  - Keys entries by absolute path and mirrors the cache to a `.peisar_cache`
    directory on disk; `PEISAR_CACHE_FORMAT=bincode` switches entries to
    compact `.bin` files
  - Recursive file watching keeps the in-memory cache and the disk mirror in
    sync; `dispose()` releases the watcher and worker threads (safe to call
    more than once)
- `Peisar.toml` static-site configuration loading:
  `peisar::ssg::ssg_config::parse_config` and the fallible
  `peisar::ssg::ssg_config::load_config`, plus the `peisarSsgConfig()`
  Node.js export
- JSON Schema for `Peisar.toml` at `schema/peisar-ssg.json`
- Markdown and asset file discovery under `peisar::ssg::files`
- Git hooks: a `commit-msg` hook enforcing the commit message convention
  (`.githooks/commit-msg`), plus `scripts/commit.sh` (`npm run commit`) and
  `scripts/install-hooks.sh` (`npm run hooks:install`)
- Agent Skills for AI coding tools, published with
  `npx skills add phothinmg/peisar`:
  `peisar-parse-markdown`, `peisar-ast-visitors`, `peisar-parser-hooks`, and
  `peisar-development`
- Integration tests that compile-check every documented Rust and Node.js API
  path in `README.md` (`tests/readme_paths.rs`)
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
  - `peisar::ssg::{cache, files, ssg_config}` — cache, discovery, SSG config
- Expanded the parser test suite (`src/markdown/ast/tests.rs`) and the
  Node.js integration tests (`test/peisar.test.js`) to cover front matter,
  visitors, hooks, and their composition
- Rewrote documentation: README plus the `contents/` guide and reference
  pages

### Fixed

- Blank lines are no longer stripped inside template literals during
  consolidation steps
- Cache disposal now reliably releases watcher and worker threads so Node
  processes exit cleanly after tests

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

[Unreleased]: https://github.com/phothinmg/peisar/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/phothinmg/peisar/compare/v0.1.0...v0.1.2
[0.1.0]: https://github.com/phothinmg/peisar/releases/tag/v0.1.0