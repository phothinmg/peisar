---
name: peisar-cache
description: Work with PeisarCache to cache a site's Markdown and assets, derive page and asset objects, configure memory and disk tiers, or watch files for changes in Rust or Node.js.
---

# PeisarCache

Use `PeisarCache` when a site build needs to discover Markdown and asset files,
read cached content, derive template-ready page/asset objects, or keep those
entries synchronized with filesystem changes. For parsing one Markdown
document, use the `peisar-parse-markdown` skill instead.

## API references

- Rust API and examples: [`docs/rust/apis.md`](../../docs/rust/apis.md), section
  `peisar::cache`.
- Node.js API and configuration tables:
  [`docs/nodejs/apis.md`](../../docs/nodejs/apis.md), section `PeisarCache`.
- Implementation and colocated tests: `src/cache/`.

Check the API reference and implementation before changing behavior; the docs
describe the supported public surface and defaults.

## Node.js

```js
const { PeisarCache } = require("peisar");
const path = require("node:path");

const cache = PeisarCache.withConfig({
  entryDir: "contents",
  assetsDir: "public",
  memory: { byteBudget: 32 * 1024 * 1024, entryBudget: 500 },
  archive: { shardBytes: 8 * 1024 * 1024, flushIntervalMs: 2000 },
});

const pages = cache.pages();
const assets = cache.assets();
const source = cache.getText(path.resolve("contents/intro.md"));
cache.dispose();
```

The constructor is `new PeisarCache(entryDir, assetsDir?, options?)`.
`PeisarCache.withConfig(config)` accepts the full configuration, including
memory limits; use the constructor options for Markdown, archive, hosting,
output-directory, and cache-directory settings.

## Rust

```rust
use peisar::cache::{CachedContent, PeisarCache, PeisarCacheConfig};
use std::path::Path;

let mut cache = PeisarCache::with_config(PeisarCacheConfig {
    entry_dir: "contents".into(),
    assets_dir: Some("public".into()),
    ..PeisarCacheConfig::default()
})?;

for page in cache.page_objects() {
    println!("{} -> {}", page.slug, page.out_file_path);
}
let path = std::env::current_dir()?.join("contents/intro.md");
if let Some(CachedContent::Text(markdown)) = cache.get(Path::new(&path)) {
    println!("{markdown}");
}
```

Use `PeisarCache::new(entry_dir)` for default configuration or
`PeisarCache::with_config(PeisarCacheConfig { .. })` to configure paths,
Markdown rendering, memory/archive budgets, hosting URLs, and output paths.
`get(path)` returns `Option<CachedContent>`: `Text(String)` for Markdown and
text assets, and `Binary(Vec<u8>)` for binary assets.

## Behavior and lifecycle

- Markdown is discovered recursively under `entryDir`; assets are optional
  and discovered under `assetsDir`. Relative paths resolve from the current
  working directory. If assets are omitted, an existing `public` directory
  is used.
- Markdown extensions include `md`, `markdown`, `mdown`, `mkdn`, `mkd`,
  `mdwn`, `mkdown`, and `ron`. Recognized assets include common image,
  script/style, font, and audio/video files. `js`, `css`, `map`, and `svg`
  are text; other recognized assets are bytes.
- Reads check the in-memory LRU first, then the disk archive. The default
  memory budget is 64 MiB with unlimited entries; `0` means unlimited for
  either memory budget.
- The archive writes binary shards and a `map.json` index under
  `.peisar-cache` by default. It flushes on the size threshold, interval,
  explicit `flush()`, or shutdown. Default shard size is 8 MiB and default
  flush interval is 2000 ms; `0` disables the corresponding trigger.
- `page_objects()` / `asset_objects()` derive objects sorted by slug.
  Node.js exposes these as `pages()` / `assets()`. Page HTML uses Peisar's
  Markdown pipeline; page metadata is extracted from YAML front matter.
- Register change callbacks with `on_change()` / `onChange()` before
  starting the watcher. Rust uses `start_watching()` and
  `stop_watching()`; Node.js uses `startWatchingJs()` and `dispose()`.
  Watch setup errors are surfaced. Stop watching and flush pending writes
  during graceful shutdown; dropping the Rust cache also joins its workers.

## Change and test workflow

1. Update the relevant cache implementation (`mod.rs`, `lru.rs`,
   `archive.rs`, `file.rs`, `obj_cache.rs`, or `base_url.rs`) and preserve
   the behavior documented above.
2. Add or update tests in the `#[cfg(test)]` module in `src/cache/mod.rs`.
   Keep Rust and Node.js API behavior aligned when changing N-API-visible
   fields or methods.
3. Update `docs/rust/apis.md` and/or `docs/nodejs/apis.md` for public API or
   default changes.
4. Run focused Rust cache tests with `cargo test cache::` and, for Node.js
   binding changes, run `npm run js:test`.
