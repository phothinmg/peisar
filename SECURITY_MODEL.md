# peisar — threat model

_Based on the current repository source and the trust model in
[`SECURITY.md`](SECURITY.md)._

## System overview

peisar is a library, not a service. It parses Markdown strings and provides
HTML, AST, and frontmatter results through a Rust crate and a Node.js native
addon. CI also builds a WASI target; the browser bundle is a separate
TypeScript implementation.

The parser API has no filesystem, watcher, archive, cache, or site-generation
surface. Frontmatter is extracted as a detected format and raw string; YAML,
TOML, and JSON are not deserialized. The main trust boundary is therefore
untrusted Markdown text entering peisar and rendered HTML or AST data being
consumed by the host application.

## Components

| Component | Code | Notes |
|---|---|---|
| Markdown parser and AST | `src/peisar_core/parsers/`, `tokens/`, `options.rs` | Parses Markdown into AST nodes, including GFM, Kramdown attributes, source spans, and optional plugin hooks. |
| HTML renderer | `src/html.rs` | Renders AST nodes to HTML; escapes text and attribute values, while raw HTML passes through. |
| Frontmatter | `src/peisar_core/frontmatter.rs`, `src/parser.rs` | Extracts YAML/TOML/JSON frontmatter as raw content; it does not parse those formats. |
| Public API and N-API bindings | `src/lib.rs`, `src/parser.rs`, `src/config.rs`, `src/peisar_core/control.rs` | Exposes `Peisar`, options, AST/render/plugin modules, and the Node.js `use()` plugin method. |
| Build and release | `.github/workflows/ci.yml`, `scripts/`, `package.json`, `napi.config.json`, `Cargo.toml` | Supply-chain and release-pipeline risks are out of scope **[maintainer]**. |

## Entry points and who reaches them

- Node.js: `new Peisar(markdown, options?)`, followed by the `ast`,
  `html`, or `frontmatter` property; `document.use(plugin)` registers
  developer-supplied parser hooks or AST visitors.
- Rust: `Peisar::new` provides the high-level parse/render API.
  `peisar::ast` also exposes AST types and parsing functions.
- Frontmatter input is part of the Markdown string. Its returned `raw` value
  is not deserialized or validated as YAML, TOML, or JSON.

Host applications may pass end-user Markdown to these APIs. Plugin callbacks
and render/parser options are supplied by the integrating developer and are
treated as developer-controlled.

## Assets (priority order)

1. **Integrity of rendered HTML** in downstream applications. Escaping bugs
   that let untrusted Markdown inject markup or script beyond documented raw
   HTML passthrough are in scope.
2. **Availability of host processes.** Crafted Markdown causing hangs,
   excessive resource use, stack exhaustion, or process-aborting panics is
   in scope **[maintainer]**.
3. **Memory safety** of the Rust parser and native addon integration.

## Attacker profiles

- **A1 – Untrusted Markdown author** (primary): submits Markdown, including
  raw frontmatter strings, to an application that parses or renders it.
  Relevant goals are XSS through escaping defects and denial of service.
- **Trusted integrating developer:** controls parser/render options and
  plugin callbacks.
- **Not considered:** supply-chain and CI attackers; the release pipeline is
  out of scope **[maintainer]**.

## Existing controls

- Text nodes, code content, titles, link/image attribute values, and
  Kramdown attribute names and values are escaped by the HTML renderer.
- Rust ownership and types are used throughout the parser and renderer.
  The native Node.js surface is provided through napi-rs.
- Parsing is performed on caller-provided strings; the parser API does not
  read paths or files.

## Out of scope / accepted risk

- **No HTML sanitization (see `SECURITY.md`):** raw HTML blocks and inline
  HTML pass through, and link/image URL schemes are not filtered
  (`javascript:` and `data:` are not rejected). Consumers rendering
  untrusted Markdown must sanitize the resulting HTML. Findings limited to
  this documented behavior are accepted risk. Escaping defects in text,
  attributes, Kramdown attributes, or generated document metadata remain in
  scope.
- **Developer-controlled inputs:** issues reachable only through trusted
  plugin callbacks or render/parser options are low priority **[maintainer]**.
- **Release pipeline and supply chain:** `.github/workflows/`, scripts,
  hooks, publishing, third-party actions, and the separate browser
  implementation are out of scope **[maintainer]**.
- **Unsupported versions:** 1.2.x and older do not receive security fixes.

## Finding tags

- `CWE` — CWE identifier for the weakness.
- `surface` — affected API surface: `npm` (N-API addon), `rust-crate`,
  `wasm` (WASI build), or a comma-separated combination.

## Open questions

- None outstanding.
