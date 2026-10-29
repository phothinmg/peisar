# peisar — threat model

_Built from the code at `phothinmg/peisar` (v1.1.x), the "Trust model" section of `SECURITY.md`, and the maintainer's answers (marked **[maintainer]**)._

## System overview

peisar is a **library**, not a service: a CommonMark/GFM/Kramdown Markdown parser in Rust, shipped as
- a Rust crate (`src/lib.rs`, `rlib`),
- a Node.js native addon via napi-rs (`--features npm`, `cdylib`, `index.cjs` / `index.mjs`),
- a WASI build (`wasm32-wasip1-threads`) and a separate TypeScript browser bundle (`browser/peisar.js`, built from `suseejs/peisar_browser` in CI).

There is no deployment infrastructure and no infrastructure repository **[maintainer]**; risk lands in the applications that embed peisar. The main trust boundary is **Markdown/YAML text → peisar → HTML/AST/JS objects consumed by the host app**. A secondary boundary is the **filesystem** touched by `PeisarCache`.

## Components

| Component | Code | Notes |
|---|---|---|
| **Markdown parser** | `src/markdown/ast/` (parsers `block.rs`, `inline.rs`, `table.rs`, `atters.rs`, tokens, `mod.rs`) | Parses untrusted text into the AST; Kramdown `{: ...}` attributes, link refs, spans. |
| **HTML renderer** | `src/markdown/html/mod.rs` | AST → HTML; escapes text and attribute values (`esc`, `esc_attr`). |
| **Front matter & YAML** | `src/frontmatter/`, `src/yaml/` | `---` block extraction; YAML via `noyalib`; `load_yaml` / `load_yaml_*_file` read files by path. |
| **JS bindings & plugins** | `src/markdown/peisar/`, `src/markdown/config/`, `src/markdown/ast/control.rs`, `src/markdown/ast/parsers/plugin.rs`, napi exports in `src/yaml`, `src/frontmatter` | `Peisar` class, `usePlugin`/`useVisitor`/`useParser` JS callbacks (`FunctionRef`), options mapping. |
| **Content cache** | `src/cache/` (`mod.rs`, `archive.rs`, `lru.rs`, `obj_cache.rs`, `file.rs`, `base_url.rs`) | `PeisarCache`: directory walk, notify watcher, bincode shard archive, `map.json`, slugs / `outFilePath`, hosting base URL, `.gitignore` edits, one `unsafe` SIGINT handler (`mod.rs` ~L506). |
| **Build & release** | `.github/workflows/ci.yml`, `scripts/`, `.githooks/`, `package.json`, `napi.config.json`, `Cargo.toml` | Out of scope **[maintainer]**. |

## Entry points and who reaches them

- `new Peisar(rawMd, options)` → `.html`, `.ast`, `.astJson`, `.frontmatter` (JS); `Document::parse`, `render_document_html` (Rust). **Reached by untrusted end users** of host apps as well as trusted site authors **[maintainer]**.
- `frontmatter(content)`, `yamlParser(str)` — same: untrusted input.
- `loadYaml(filePath)` (JS) / `load_yaml_value_file`, `load_yaml_config_file` (Rust) — path chosen by the trusted developer.
- `PeisarCache(entryDir, assetsDir, options)` / `with_config`, `startWatchingJs`, `getPage`, `pages`, `mapJson`, `dispose` — directories and options are chosen only by the trusted developer/site author **[maintainer]**. File contents under those dirs may be Markdown from contributors, but dirs/options are not attacker-steerable.
- JS plugin callbacks (`visitBlock`, `parseBlock`, …) — developer code; trusted.

## Assets (priority order)

1. **Integrity of rendered HTML** in downstream apps — escaping bugs that let untrusted Markdown inject markup/script **beyond** the documented raw-HTML passthrough.
2. **Availability** of host processes — crafted Markdown/YAML causing hangs (super-linear parsing), unbounded recursion / stack overflow, memory blow-up, or panics that abort a Node process. **In scope [maintainer].**
3. **Memory safety** of the native addon (Rust + napi FFI + `unsafe` signal handler).
4. **Host filesystem** around `PeisarCache` — matters only when untrusted *content* (not dirs/options) can steer reads/writes **[maintainer]**.

## Attacker profiles

- **A1 – Untrusted Markdown/YAML author** (primary): submits Markdown/front matter to an app that renders it with peisar **[maintainer]**. Goals: XSS via escaping bugs, DoS of the rendering process.
- **A2 – Content contributor to a site repo**: controls Markdown file contents under a `PeisarCache` entry dir; does **not** control directories or options **[maintainer]**.
- Trusted: the integrating developer/site author (options, cache directories, `loadYaml` paths, plugins).
- Not considered: supply-chain / CI attackers (release pipeline out of scope **[maintainer]**).

## Existing controls

- Text content and attribute values are HTML-escaped (`esc`, `esc_attr`, `esc_no_quote` in `html/mod.rs`); code-block content escaped.
- Link/image URLs attribute-escaped.
- Archive writes are atomic (temp + rename); archive dir resolved under cwd.
- Memory-safe Rust everywhere except the SIGINT handler.

## Out of scope / accepted risk

- **No HTML sanitization (from `SECURITY.md`)**: raw HTML blocks and inline HTML pass through unescaped, and `javascript:` / `data:` URLs are not scheme-filtered — documented, intended CommonMark behaviour; consumers must sanitize. Findings that only restate this are accepted risk. Escaping bugs outside it (text, attribute values, Kramdown attributes, code info strings, titles, attribute-context breakouts) stay in scope.
- **Release pipeline and supply chain** (`.github/workflows/`, `scripts/`, `.githooks/`, publishing, third-party actions, `suseejs/peisar_browser`) **[maintainer]**.
- **Developer-controlled inputs**: `PeisarCache` directories/options (`entryDir`, `assetsDir`, `outDir`, cache dir, hosting config), `loadYaml` paths, JS plugins — issues reachable only through these are low priority **[maintainer]**.
- Unsupported versions (0.2.x and older).

## Finding tags

- `CWE` — CWE id of the weakness.
- `surface` — affected API surface: `npm` (napi addon), `rust-crate`, `wasm` (WASI build), or a comma-separated combination.

## Open questions

- None outstanding.

