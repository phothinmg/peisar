# Peisar documentation

This directory holds the repository-hosted documentation for Peisar.

These docs are intentionally kept outside the package registry metadata so
they can be updated between npm and crates.io releases.

## Guides

### Node.js

- [Overview](./nodejs/README.md)
- [Next.js](./nodejs/nextjs.md)
- [Vite](./nodejs/vite.md)
- [WASM environments](./nodejs/wasm.md)
- [Node.js API reference](./nodejs/apis.md)

### Browser

- [Browser usage example](../examples/browser/example.md)
- [Runnable browser example](../examples/browser/index.html)

### Rust

- [Overview](./rust/README.md)
- [Rust API reference](./rust/apis.md)

### Current API reference

The API references describe the single-document parser and Node.js plugin
interface. Rust's `ast` and `plugin` modules are available without Cargo
feature flags; the `npm` feature is only needed to build the Node.js bindings:

- [Node.js API](./nodejs/apis.md)
- [Rust API](./rust/apis.md)

## Reference

- [Benchmark results](./benchmarks.md)
- [Markdown language spec](./markdown-spec.md)

## Releases

- [Release notes](./release-notes/2.0.1.md) for the current version
- [Release notes](./release-notes/2.0.0.md) for the previous major release
- [Full changelog](../CHANGELOG.md)
- [Security policy](../SECURITY.md) and [threat model](../SECURITY_MODEL.md)

## Package entry points

- Top-level package page: [README.md](../README.md)
- API docs on docs.rs: <https://docs.rs/peisar>
