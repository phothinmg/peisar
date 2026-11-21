# Installation

Peisar is distributed dual-natively as a Rust crate on crates.io and a native Node.js addon (powered by N-API) on npm.

## Node.js

To install the native Node.js package in your project, use your preferred package manager:

```sh
npm i peisar
```

The package contains pre-built native bindings for major platforms, eliminating the need for local compilation during normal installation.

## Rust

To add Peisar to a Rust project, include it in your `Cargo.toml`. No special feature flags are required for standard Rust usage; the default feature set compiles the core engine and high-level wrappers:

```toml
[dependencies]
peisar = "2.1.0"
```

The `npm` Cargo feature is reserved strictly for building the N-API native Node.js bindings and should not be enabled in pure Rust projects.

## Supported Platforms and Targets

### Node.js Targets

The npm package supports pre-compiled N-API binaries targeting:

- **Linux**: x64, arm64 (GNU and musl)
- **macOS**: x64, arm64 (Apple Silicon)
- **Window**:  x64, arm32, arm64

### Rust Targets

As a pure Rust crate, Peisar compiles to any target supported by the Rust compiler (`rustc`) supporting `std` (edition 2021+), including embedded systems, WebAssembly targets (`wasm32-wasip1`), and serverless environments.