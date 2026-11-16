# Contributing to Peisar

Thank you for your interest in contributing to Peisar! This document covers
everything you need to set up a development environment, make changes, and
submit them for review.

## Prerequisites

- **Node.js** 24 or later (for the native bindings and tests)
- **Rust** stable toolchain (install with [rustup](https://rustup.rs/))
- **npm** — dependencies are managed by `@napi-rs/cli`

Install the dependencies:

```sh
npm install
```

## Project layout

Peisar builds as both a Rust library (`rlib`) and a native Node.js addon
(`cdylib`). The Rust crate provides the parser, AST, renderer, frontmatter,
and plugin APIs:

```text
src/
├── lib.rs                    public modules and root re-exports
├── config.rs                 parser and rendering options
├── html.rs                   HTML rendering
├── parser.rs                 high-level Peisar API and N-API binding
└── peisar_core/
    ├── frontmatter.rs        frontmatter extraction and types
    ├── control.rs            plugin interfaces and visitor controls
    ├── options.rs            AST parsing options
    ├── parsers/              block, inline, attribute, and plugin parsers
    └── tokens/               AST nodes and source spans
```

`npm run test` builds the addon in a temporary `tests/` directory and runs
the Node integration tests there. The test file is kept as `.txt` so
`node --test` does not auto-discover it. Agent Skills for AI coding tools
are in `skills/`.

The crate's public APIs are `peisar::{Peisar, PeisarOptions}`,
`peisar::ast`, `peisar::render`, and `peisar::plugin`; no Cargo feature is
needed for Rust use. The high-level `Peisar` API exposes `ast()`, `html()`,
`frontmatter()`, and `use_plugin()`. Its Node.js binding provides `ast`,
`html`, and `frontmatter` properties and a `use()` method for plugins.
Frontmatter contains its detected type and raw contents; it is not
deserialized. The `npm` feature enables N-API bindings. Generated
TypeScript declarations (`index.d.ts`) come from the N-API build — never
hand-edit them.

## Development workflow

Build and test the native addon (this also regenerates the package loader and
TypeScript declarations):

```sh
npm run test
```

Run all checks before opening a pull request:

```sh
npm run test          # after changing N-API-annotated Rust code
                      # Node.js integration tests
npm run build         # package build (index.cjs / index.mjs / index.d.ts)
cargo test            # Rust parser and integration tests
cargo fmt             # formatting (required by the commit helper)
```

`npm run test` compiles a fresh addon in a temporary `tests/` directory and
runs the Node.js integration tests against it — it covers both the build and
the test step after N-API-visible Rust types or methods change.

### Fixing a GPG key database lock

`npm run fix:gpg` is a local recovery script for the package author's Fedora
Workstation. It fixes a GPG key database lock by restarting the GPG agent and
removing the lock file. Do not run this command on other systems.

## Change guidelines

- Keep the public APIs in `peisar::ast`, `peisar::render`, and
  `peisar::plugin` aligned with their Node.js bindings.
- Use `Peisar.use()` for Node.js plugin integrations. A plugin can combine
  AST visitors (`visitBlock`, `visitInline`) and parser hooks (`parseBlock`,
  `parseInline`).
- Add or update Node.js integration tests in `scripts/peisar.test.txt` when
  changing the N-API surface; run `cargo test` for Rust changes.

## Commit message convention

Commits follow the `<Type> : <message>` format. The project uses these emoji
types:

| Type | Meaning |
| --- | --- |
| ⭐ feat | New feature |
| 🐛 bug | Bug fix |
| 🎨 modified | Code or style change |
| 🔒 security | Security fix |
| 👕 refactor | Refactoring |
| ⚠️ deprecated | Deprecation |
| 🚀 release | Release chore |
| ✅ tests | Tests |
| 📝 docs | Documentation |

A `commit-msg` Git hook enforces this format. To install the hooks and the
interactive commit helper:

```sh
npm run hooks:install   # sets git hooksPath to .githooks
npm run commit          # prompts for type and message, formats, commits, pushes
```

## Submitting changes

1. Fork the repository and create a branch from `main`.
2. Make your changes and add tests.
3. Run the full check list (`cargo fmt`, `npm run test`,
   `cargo test`) and make sure everything passes.
4. Commit using the conventional format above.
5. Open a pull request against `main` with a short description of the change
   and the tests you added.

On published releases, CI runs the Node.js test suite on Ubuntu, macOS, and
Windows, and builds the addon for nine targets (GNU/MUSL Linux x64/arm64,
macOS x64/arm64, Windows x64/arm64, and WASI threads). Releases are
tagged as `v<version>` following [Semantic Versioning](https://semver.org/spec/v2.0.0.html),
and notable changes are recorded in `CHANGELOG.md`.

## Reporting issues

Open a GitHub issue with:

- What you did (minimal reproduction steps or code snippet)
- What you expected and what actually happened
- Peisar version, Node.js version, and OS

For security vulnerabilities, do not open a public issue — follow the private
reporting process in [SECURITY.md](SECURITY.md) instead.

## License

By contributing to Peisar, you agree that your contributions are licensed
under the Apache-2.0 license (see `LICENSE`).