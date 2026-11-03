# peisar — threat model

_Built from the code at `phothinmg/peisar` (v2.0.1), the "Trust model" section of `SECURITY.md`, and the maintainer's answers (marked **[maintainer]**)._

## System overview

peisar is a **library**, not a service: a CommonMark/GFM/Kramdown Markdown parser in Rust, shipped as
- a Rust crate (`src/lib.rs`, `rlib`) with public `ast` and `plugin` modules,
- a Node.js native addon via napi-rs (`--features npm`, `cdylib`, `index.cjs` / `index.mjs`),
- a WASI build (`wasm32-wasip1-threads`) and a separate TypeScript browser bundle (`browser/peisar.js`, built from `suseejs/peisar_browser` in CI).

Since 2.0.0 peisar is a **pure Markdown parser**: it accepts Markdown/YAML strings and returns HTML, AST, or JSON values. It has no filesystem, watcher, archive, theme, or site-generation surface. There is no deployment infrastructure and no infrastructure repository **[maintainer]**; risk lands in the applications that embed peisar. The single trust boundary is **Markdown/YAML text → peisar → HTML/AST/JS objects consumed by the host app**.

## Components

| Component | Code | Notes |
|---|---|---|
| **Markdown parser & AST** | `peisar_ast` dependency (`parsers/`, `tokens/`, `options.rs`, `control.rs`) | Parses untrusted text into the AST; Kramdown `{: ...}` attributes, link refs, source spans, parser hooks and visitors. |
| **HTML renderer** | `src/core/html.rs` (delegating to the `peisar_ast` renderer) | AST → HTML; escapes text and attribute values (`esc`, `esc_attr`, `esc_no_quote`). |
| **Front matter & YAML** | `peisar_ast` dependency and Node.js package helpers | `---` block extraction; YAML via `noyalib`; the Node.js `loadYaml` helper reads a file by path. |
| **JS bindings & plugins** | `src/core/parser.rs`, `src/core/config.rs`, plugin plumbing in `peisar_ast::control` | `Peisar` class, the `use()` plugin callbacks (`FunctionRef`), options mapping. |
| **Build & release** | `.github/workflows/ci.yml`, `scripts/`, `.githooks/`, `package.json`, `napi.config.json`, `Cargo.toml` | Out of scope **[maintainer]**. |

## Entry points and who reaches them

- `new Peisar(rawMd, options)` → `.html`, `.ast`, `.astJson`, `.frontmatter`; `document.use(plugin)` (JS); `Document::parse` and `Peisar::html()` (Rust). **Reached by untrusted end users** of host apps as well as trusted site authors **[maintainer]**.
- `frontmatter(content)`, `yamlParser(str)` — same: untrusted input.
- `loadYaml(filePath)` (JS) — path chosen by the trusted developer. (Read failures panic in the N-API wrapper; parse failures are returned as errors.)
- JS plugin callbacks (`visitBlock`, `parseBlock`, …) — developer code; trusted.

All of these operate on strings supplied by the caller. No peisar API walks directories, watches files, or writes to disk.

## Assets (priority order)

1. **Integrity of rendered HTML** in downstream apps — escaping bugs that let untrusted Markdown inject markup/script **beyond** the documented raw-HTML passthrough.
2. **Availability** of host processes — crafted Markdown/YAML causing hangs (super-linear parsing), unbounded recursion / stack overflow, memory blow-up, or panics that abort a Node process. **In scope [maintainer].**
3. **Memory safety** of the native addon (Rust + napi FFI). The crate and its `peisar_ast` dependency contain no `unsafe` code; the 1.x SIGINT signal handler was removed together with the cache module.

## Attacker profiles

- **A1 – Untrusted Markdown/YAML author** (primary): submits Markdown/front matter to an app that renders it with peisar **[maintainer]**. Goals: XSS via escaping bugs, DoS of the rendering process.
- Trusted: the integrating developer (options, `loadYaml` paths, plugins).
- Not considered: supply-chain / CI attackers (release pipeline out of scope **[maintainer]**).

## Existing controls

- Text content and attribute values are HTML-escaped (`esc`, `esc_attr`, `esc_no_quote` in `src/core/html.rs`); code-block content escaped.
- Link/image URLs attribute-escaped.
- Memory-safe Rust throughout; no `unsafe` blocks in this crate or its parser/YAML dependencies.

## Out of scope / accepted risk

- **No HTML sanitization (from `SECURITY.md`)**: raw HTML blocks and inline HTML pass through unescaped, and `javascript:` / `data:` URLs are not scheme-filtered — documented, intended CommonMark behaviour; consumers must sanitize. Findings that only restate this are accepted risk. Escaping bugs outside it (text, attribute values, Kramdown attributes, code info strings, titles, attribute-context breakouts) stay in scope.
- **Release pipeline and supply chain** (`.github/workflows/`, `scripts/`, `.githooks/`, publishing, third-party actions, `suseejs/peisar_browser`) **[maintainer]**.
- **Developer-controlled inputs**: `loadYaml` paths, JS plugins, parse/render options — issues reachable only through these are low priority **[maintainer]**.
- Unsupported versions (1.2.x and older).

## Finding tags

- `CWE` — CWE id of the weakness.
- `surface` — affected API surface: `npm` (napi addon), `rust-crate`, `wasm` (WASI build), or a comma-separated combination.

## Open questions

- None outstanding.
