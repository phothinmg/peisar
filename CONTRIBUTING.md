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
(`cdylib`). The high-level wrapper lives in this crate; the AST and YAML
implementations are provided by the `peisar_ast` and `peisar_yaml`
dependencies:

```text
src/
├── ast.rs                    feature-gated AST re-exports
├── core/
│   ├── config.rs             parser and rendering options
│   ├── html.rs               HTML rendering integration
│   └── parser.rs             Peisar wrapper and N-API binding
├── html.rs                   feature-gated renderer re-exports
├── plugin.rs                 feature-gated plugin re-exports
├── yaml.rs                   feature-gated YAML re-exports
└── lib.rs                    public modules and root re-exports

scripts/peisar.test.txt       Node.js integration tests

`npm run js:test` builds the addon in a temporary directory and runs the Node
integration tests there; the file is kept as `.txt` so `node --test` does not
auto-discover it.
skills/                       Agent Skills for AI coding tools
```

Rust consumers use `peisar::ast`, `peisar::html`, `peisar::plugin`, and
`peisar::yaml` when enabling those Cargo features. `Peisar` and its options
are re-exported at the crate root. The generated TypeScript declarations
(`index.d.ts`) are produced by the N-API build — never hand-edit them.

## Development workflow

Build and test the native addon (this also regenerates the package loader and
TypeScript declarations):

```sh
npm run js:test
```

Run all checks before opening a pull request:

```sh
npm run js:test        # after changing N-API-annotated Rust code
                      # Node.js integration tests
npm run build          # package build (index.cjs / index.mjs / index.d.ts)
cargo test             # Rust parser and integration tests
cargo fmt              # formatting (required by the commit helper)
```

`npm run js:test` compiles a fresh addon in a temporary directory and runs the
Node.js integration tests against it — it covers both the build and the test
step after N-API-visible Rust types or methods change.

### Fixing a GPG key database lock

`npm run fix:gpg` is a local recovery script for the package author's Fedora
Workstation. It fixes a GPG key database lock by restarting the GPG agent and
removing the lock file. Do not run this command on other systems.

## Change guidelines

- The parser, AST, renderer, plugin, and YAML implementations live in their
  respective dependency crates. Keep this crate's feature-gated re-exports
  and N-API wrapper aligned with those dependency APIs.
- Use `Peisar.use()` for Node.js plugin integrations.
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
3. Run the full check list (`cargo fmt`, `npm run js:test`,
   `cargo test`) and make sure everything passes.
4. Commit using the conventional format above.
5. Open a pull request against `main` with a short description of the change
   and the tests you added.

CI runs the Node.js test suite on Ubuntu, macOS, and Windows, and builds
the native addon for eight targets (GNU/MUSL Linux x64/arm64, macOS
x64/arm64, Windows x64/arm64) whenever a release is published. Releases are
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