# Introduction

**Peisar** is a high-performance Markdown parser and renderer engineered in Rust, providing native support for CommonMark, GitHub Flavored Markdown (GFM), Kramdown block attributes, and multi-format frontmatter extraction. Designed with a Rust-first core architecture, Peisar exposes its functionality natively to Rust applications via crates.io and to JavaScript/TypeScript environments through Node.js N-API bindings and WASM targets.

- CommonMark blocks and inlines, including HTML and reference links
- GFM tables, strikethrough, task lists, and autolinks
- Kramdown block attributes such as `{:#intro .lead key="value"}`
- YAML, TOML, and JSON frontmatter extraction (raw content, not deserialized)
- AST visitors and parser hooks through `Peisar.use()`
- Native Node.js addon and Rust library surfaces

## Get started

- [Install Peisar](install.md)
- [Quick Start](quick-start.md)
- [Node.js guides](docs/nodejs/README.md)
- [Rust guides](docs/rust/README.md)
- [API references](docs/nodejs/apis.md)

## Contributing

Contributions are what make the open-source community such an amazing place to learn, inspire, and create. **Any contributions you make are greatly appreciated!**

Whether you're fixing a typo, improving documentation, reporting a bug, or proposing a new feature, we'd love your help. 

1. Check out our [Contributing Guidelines](https://github.com/phothinmg/peisar/blob/main/CONTRIBUTING.md) for setup instructions and style rules.
2. Browse our [open issues](https://github.com/phothinmg/peisar/issues) — look for the **`good first issue`** label if you're new to the project.
3. Fork the repo, create your branch, and open a Pull Request!

Not sure where to start? Feel free to open a [Discussion](https://github.com/phothinmg/peisar/discussions) or ask a question on an issue. We're happy to help!


## License

Except as otherwise noted, the content of this documentation is licensed under the 
[CC BY 4.0][cc-by-4] and source code  are licensed under the
[Apache-2.0][license]

[license]: https://github.com/phothinmg/peisar/blob/main/LICENSE
[ptm]: https://github.com/phothinmg
[cc-by-4]:https://creativecommons.org/licenses/by/4.0/