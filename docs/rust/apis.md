# Rust API reference

This document summarizes the current public Rust-facing API layout.

## `peisar::frontmatter`

Primary entry point:

```rust
use peisar::frontmatter::frontmatter;
```

Returns `FrontmatterResult`, which provides:

- `yaml_data() -> Option<&serde_json::Value>`
- `pure_markdown_content() -> &str`
- `into_parts() -> (String, Option<serde_json::Value>)`

Use it when you want front matter separated before parsing Markdown further.

## `peisar::yaml`

Dynamic YAML helpers:

- `parse_yaml_value(&str) -> Result<serde_json::Value, String>`
- `load_yaml_value_file(path) -> Result<serde_json::Value, String>`

Typed config helpers:

- `parse_yaml_config<T>(&str) -> Result<T, String>`
- `load_yaml_config_file<T>(path) -> Result<T, String>`

## `peisar::markdown::ast`

Key exports:

- `AstOptions`
- `Document`
- `PluginFactory`
- `ParserHooks`
- `visit_document_mut`
- `md_to_ast_with_hooks`

Notable behavior:

- `Document::parse(...)` is the normal entry point for parsing Markdown.
- GFM and Kramdown parsing are configured through `AstOptions`.
- `PluginFactory` is the current unified extension trait for parser hooks and
  visitor callbacks.
- Deprecated compatibility traits `AstVisitor` and `AstParser` still exist.

### Example

```rust
use peisar::markdown::ast::{AstOptions, Document};

let document = Document::parse("# Hello\n", &AstOptions::default(), None);
assert_eq!(document.children.len(), 1);
```

## `peisar::markdown::html`

Key exports:

- `RenderOptions`
- `render_document_html(&Document, Option<RenderOptions>) -> String`
- `AstToHtml`

Use `RenderOptions { fragment: true, .. }` when you want just the rendered
body content instead of a full HTML document.

## `peisar::markdown::config`

Key exports:

- `PeisarOptions`
- `get_options`

`PeisarOptions` is the shared high-level option object used by the Node-facing
wrapper and the Rust `markdown::peisar::Peisar` type.

## `peisar::markdown::peisar`

Key type:

- `Peisar`

Notable methods:

- `Peisar::new(markdown, options)`
- `ast()`
- `html()`
- `frontmatter()`
- `ast_json()`

This wrapper is the closest Rust equivalent to the Node.js `Peisar` class.

## `peisar::cache`

`PeisarCache` loads Markdown and optional assets into an in-memory LRU and
keeps evicted entries readable through a background disk archive.

```rust
use peisar::cache::{CachedContent, PeisarCache, PeisarCacheConfig};

let mut cache = PeisarCache::with_config(PeisarCacheConfig {
    entry_dir: "contents".into(),
    assets_dir: Some("public".into()),
    ..PeisarCacheConfig::default()
})?;

for page in cache.page_objects() {
    println!("{} -> {}", page.slug, page.out_file_path);
}
let intro = std::env::current_dir()?.join("contents/intro.md");
if let Some(CachedContent::Text(markdown)) = cache.get(&intro) {
    println!("{markdown}");
}

cache.start_watching()?;
cache.stop_watching(); // Stops watching and flushes pending archive writes.
# Ok::<(), std::io::Error>(())
```

### Configuration

- `PeisarCache::new(entry_dir)` uses default settings.
- `PeisarCache::with_config(PeisarCacheConfig { .. })` configures the
  directories, Markdown options, memory/archive budgets, hosting, output
  directory, and archive directory.
- `entry_dir` defaults to `"contents"`. If `assets_dir` is omitted, an
  existing `public` directory under the current working directory is used.
- `MemoryConfig` defaults to a 64 MiB byte budget and unlimited entries.
  Set either budget to `0` for unlimited.
- `ArchiveConfig` defaults to 8 MiB shards and a 2-second flush interval.
  Set either threshold to `0` to disable that trigger.
- `out_dir` defaults to `".peisar"` and `cache_dir` to `".peisar-cache"`,
  both relative to the current working directory.

Markdown files with extensions `md`, `markdown`, `mdown`, `mkdn`, `mkd`,
`mdwn`, `mkdown`, or `ron` are discovered recursively. The optional assets
directory recognizes common image, script/style, font, and audio/video file
types. Markdown and textual assets are stored as text; other recognized
assets are stored as bytes.

### Reading and derived objects

- `get(path)` checks memory first, then the archive, and returns
  `Option<CachedContent>`.
- `page_objects()` and `asset_objects()` derive template-ready values sorted
  by slug. `page_object_for(path)` and `asset_object_for(path)` derive one.
- `markdown_files()` and `asset_files()` list absolute cached paths.
- `pages()` and `assets()` are the corresponding Node.js-facing methods.
- The memory budgets limit the hot in-memory set. Archive-backed entries
  remain readable after eviction.

`PageObject` includes its rendered HTML, slug, output and input paths, and
page metadata (`layout`, `title`, `summary`, `tags`, `publish_date`, and
additional front matter). `AssetsObject` includes its slug, paths, extension,
type, and raw content; binary asset content is base64-encoded in its `raw`
field.

### Watching and archive lifecycle

Call `start_watching()` to keep the cache synchronized with changes under the
entry and asset directories. Register callbacks with `on_change()` and remove
them with `off_change(id)`; each callback receives a `CacheChangeEvent` with
the absolute `path`, `kind` (`"create"`, `"modify"`, `"remove"`, or
`"other"`), and `is_markdown` fields. Watcher setup errors are returned from
`start_watching()`.

The archive worker writes binary shards and a `map.json` index in the cache
directory. `archived()` lists flushed paths, `map_json()` returns the current
index, and `flush()` blocks until pending writes finish. `stop_watching()`
stops the watcher and flushes pending writes; dropping the cache also flushes
and joins its workers.

## Choosing an API

- Use `markdown::ast` + `markdown::html` for lower-level parsing and rendering.
- Use `markdown::peisar::Peisar` for a higher-level single-document wrapper.
- Use `cache::PeisarCache` for filesystem-backed Markdown and asset caching.
- Use `frontmatter` or `yaml` when you only need metadata/config handling.
