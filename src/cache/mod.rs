//! Memory-first cache of a site's Markdown and asset files.
//!
//! [`PeisarCache`] loads every markdown file under an entry directory
//! (plus optional assets) into an in-memory LRU store, derives
//! template-ready [`PageObject`] / [`AssetsObject`] values on demand, and
//! — in the background — flushes content into a binary shard archive
//! under `.peisar-cache` (with a `map.json` search index). Disk is *not*
//! the primary store: reads hit memory first and only fall through to the
//! archive for entries the LRU has evicted.
//!
//! # JS usage (napi-rs surface)
//!
//! ```js
//! const cache = new PeisarCache("contents", "public", {
//!   markdown: { fragment: true },
//!   archive: { shardBytes: 8 * 1024 * 1024, flushIntervalMs: 2000 },
//!   hosting: { provider: "githubPages", repo: "site" },
//!   outDir: "out",
//! });
//! cache.pages();                    // PageObject[] — for Liquid themes
//! cache.getPage("docs/intro.md");   // single page object (or null)
//! cache.assets();                    // AssetsObject[]
//! cache.mapJson();                   // the map.json document (search index)
//! cache.archived();                  // paths flushed to the disk archive
//! cache.startWatchingJs();           // keep everything in sync with the FS
//! cache.dispose();                   // stop watching and flush the archive
//! ```
//!
//! While watching, every file operation logs one pretty line to stderr —
//! `created docs/intro.md` (ANSI-colored on terminals, path relative to
//! the project root). Duplicate events from one save coalesce, and
//! filesystem noise is not logged.
//!
//! # Rust usage
//!
//! ```no_run
//! use peisar::cache::{CachedContent, PeisarCache, PeisarCacheConfig};
//! use std::path::Path;
//!
//! # fn main() -> std::io::Result<()> {
//! let mut cache = PeisarCache::with_config(PeisarCacheConfig {
//!     entry_dir: "contents".into(),
//!     assets_dir: Some("public".into()),
//!     ..PeisarCacheConfig::default()
//! })?;
//! for path in cache.markdown_files() {
//!     if let Some(CachedContent::Text(md)) = cache.get(Path::new(&path)) {
//!         println!("{} ({} bytes)", path, md.len());
//!     }
//! }
//! cache.start_watching()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Discovery rules
//!
//! - *Markdown*: files under the entry directory whose extension is `md`,
//!   `markdown`, `mdown`, `mkdn`, `mkd`, `mdwn`, `mkdown`, or `ron`; cached
//!   as UTF-8 text.
//! - *Assets*: files under the assets directory with a known image, script,
//!   style, font, or audio/video extension. Binary types are cached as raw
//!   bytes; `js`, `css`, `map`, and `svg` are cached as UTF-8 text.
//!
//! # Tiering
//!
//! 1. **Memory (hot)** — an LRU bounded by `byteBudget`/`entryBudget` in
//!    `memory` config (or the constructor options). All reads start here.
//!
//! 2. **Disk archive (cold)** — a background worker buffers upserts and
//!    flushes binary shards under `.peisar-cache` when the pending content
//!    crosses `shardBytes`, on the `flushIntervalMs` tick, or on
//!    flush/dispose/drop. `map.json` maps every flushed path to its shard
//!    location and (for markdown) its parsed front matter.
//!
//! Entries evicted from memory stay readable via the archive
//! (`getText`/`getBinary` fall through), so the byte budget is a *hot set*
//! limit, not a correctness limit.
//!
//! # Threading and lifecycle
//!
//! The archive worker and file watcher run on background threads. JS
//! consumers call `dispose()` (or drop the cache in Rust) to stop the
//! watcher, flush pending archive writes, and join the workers. Rust
//! consumers may also call [`PeisarCache::stop_watching`] for the same
//! graceful shutdown — e.g. after a `Ctrl+C` interrupt.

mod archive;
mod base_url;
mod file;
mod lru;
mod obj_cache;

pub use archive::{ARCHIVE_DIR_NAME, ArchiveHandle};
pub use base_url::{BaseDirInfo, HostingProvider};
pub use lru::LruCache;
pub use obj_cache::{
    AssetsObject, ObjectCache, PageObject, asset_file_type, asset_out_file, asset_slug,
    extra_frontmatter_fields, file_name_no_ext, markdown_slug, page_out_file,
};

use crate::frontmatter::frontmatter;
use crate::markdown::ast::visitor::visit_document_mut;
use crate::markdown::ast::{
    AstOptions, ParseHooks, RegisteredParser, RegisteredVisitor, md_to_ast_with_hooks,
};
#[cfg(feature = "npm")]
use crate::markdown::ast::{Parser, Visitor};
use crate::markdown::config::{PeisarOptions, get_options};
use crate::markdown::html::{RenderOptions, render_document_html};
use file::{
    ASSET_BINARY_EXTENSIONS, ASSET_EXTENSIONS, MARKDOWN_EXTENSIONS, collect_asset_files,
    collect_markdown_files,
};
#[cfg(feature = "npm")]
use napi::bindgen_prelude::*;
#[cfg(feature = "npm")]
use napi_derive::napi;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// ANSI color escape sequences for the watcher's pretty logs (disabled
/// automatically when stderr is not an interactive terminal).
const LOG_STYLE_DIM: &str = "\u{1b}[2m";
const LOG_STYLE_GREEN: &str = "\u{1b}[32m";
const LOG_STYLE_YELLOW: &str = "\u{1b}[33m";
const LOG_STYLE_RED: &str = "\u{1b}[31m";
const LOG_STYLE_RESET: &str = "\u{1b}[0m";

/// `true` when the watcher's pretty logs may use ANSI colors — stderr
/// must be an interactive terminal. Captured/piped output (CI logs, test
/// runners) stays plain.
fn log_use_color() -> bool {
    io::stderr().is_terminal()
}

/// Build one pretty watcher line: `<label> <path>` with the label
/// colored by change kind (when `color` is on). Paths are shown relative
/// to `root` when possible, falling back to the absolute path for files
/// outside it.
fn format_change_line(kind: &str, path: &Path, root: &Path, color: bool) -> String {
    let (label, style) = match kind {
        "create" => ("created", LOG_STYLE_GREEN),
        "modify" => ("modified", LOG_STYLE_YELLOW),
        "remove" => ("removed", LOG_STYLE_RED),
        _ => ("changed", LOG_STYLE_DIM),
    };
    let rel = pretty_path(path, root);
    if color {
        format!("{style}{label}{LOG_STYLE_RESET} {rel}")
    } else {
        format!("{label} {rel}")
    }
}

/// Print one pretty watcher line to stderr — see
/// [`PeisarCache::start_watching`] for the format.
fn log_change_pretty(kind: &str, path: &Path, root: &Path) {
    eprintln!("{}", format_change_line(kind, path, root, log_use_color()));
}

/// Window within which consecutive events for the same path count as
/// one operation. Shell redirects, editors, and atomic saves emit
/// several events for a single save — only the first within this
/// window logs.
const LOG_DEDUPE_WINDOW: Duration = Duration::from_secs(1);

/// `true` when a path may ever enter the cache (and therefore deserves
/// a pretty log line): markdown and asset extensions are the same
/// allow-lists the cache admits content by, so editor side files
/// (`.tmp`, swap files, lock files, …) never log.
fn log_trackable(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            let ext_l = ext.to_lowercase();
            MARKDOWN_EXTENSIONS.contains(&ext_l.as_str())
                || ASSET_EXTENSIONS.contains(&ext_l.as_str())
        })
        .unwrap_or(false)
}

/// Decide whether a watcher event gets a pretty log line: each path
/// logs at most once per [`LOG_DEDUPE_WINDOW`] (create/remove included —
/// renames and atomic saves fire several events per operation) and
/// only for files the cache could ever track. Events are delivered to
/// `onChange` subscribers either way — this only filters the stderr log.
fn should_log_change(path: &Path, last_log_at: &HashMap<PathBuf, Instant>, now: Instant) -> bool {
    log_trackable(path)
        && !last_log_at
            .get(path)
            .is_some_and(|at| now.duration_since(*at) < LOG_DEDUPE_WINDOW)
}

/// Classify a watcher event from the **actual filesystem state** rather
/// than the raw notify event kind. Editors and atomic saves surface as
/// rename events ("modify"/"other" on the notify side), so the raw kind
/// cannot be trusted for a correct `created` / `modified` / `removed`
/// label:
///
/// - the file **exists** and was **already known to the cache** →
///   `modify` (the entry is being updated in place);
/// - the file **exists** and is **unknown** → `create`;
/// - the file is **gone** → `remove`.
///
/// `known` here means present in the cache tiers — exactly the paths a
/// create is allowed to add.
fn classify_change(memory: &LruCache, archive: &ArchiveHandle, path: &Path) -> &'static str {
    let known = memory.get(path).is_some() || archive.read(path).is_some();
    if path.exists() {
        if known { "modify" } else { "create" }
    } else {
        "remove"
    }
}

/// Render `path` relative to `root` when it lives underneath it
/// (`/project/contents/docs/a.md` → `contents/docs/a.md`), otherwise as
/// the lossy absolute path.
fn pretty_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .map(|rel| rel.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

/// A file-change event delivered to change callbacks
/// (see [`PeisarCache::on_change`]).
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Clone)]
pub struct CacheChangeEvent {
    /// Absolute path of the changed file.
    pub path: String,
    /// `"create"`, `"modify"`, `"remove"`, or `"other"`.
    pub kind: String,
    /// `true` when the file is under the markdown entry directory.
    pub is_markdown: bool,
}

/// An entry stored in the cache: raw UTF-8 text (markdown and textual
/// assets) or raw bytes (binary assets). Exposed to JavaScript so JS
/// consumers can branch on the variant. The serde derives back the
/// bincode round-trip through the disk archive.
#[cfg_attr(feature = "npm", napi)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CachedContent {
    /// UTF-8 text content (markdown files and textual assets).
    Text(String),
    /// Raw bytes (binary assets like images or fonts).
    Binary(Vec<u8>),
}

/// Cache-wide configuration: directories, markdown options, memory/archive
/// budgets, hosting/baseUrl resolution, and the build output directory.
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Clone)]
pub struct PeisarCacheConfig {
    /// Entry directory holding the markdown files (absolute, or relative
    /// to the current working directory).
    pub entry_dir: String,
    /// Optional assets directory (defaults to `public` under the cwd when
    /// omitted).
    pub assets_dir: Option<String>,
    /// Markdown parsing/rendering options applied to every page render
    /// (GFM/Kramdown toggles, `fragment`, title, …). Custom visitors and
    /// parsers are registered separately via `useVisitor` / `useParser`.
    pub markdown: Option<PeisarOptions>,
    /// Memory-tier budgets (LRU eviction). Defaults: 64 MiB bytes,
    /// unlimited entries.
    pub memory: Option<MemoryConfig>,
    /// Disk-archive tier thresholds. Defaults: 8 MiB shards, 2 s flush.
    pub archive: Option<ArchiveConfig>,
    /// Static-hosting baseUrl resolution (GitHub Pages/Vercel/…).
    pub hosting: Option<HostingConfig>,
    /// Build output directory used for `outFilePath` fields (default
    /// `".peisar"`).
    pub out_dir: Option<String>,
}

impl Default for PeisarCacheConfig {
    fn default() -> Self {
        Self {
            entry_dir: "contents".to_string(),
            assets_dir: None,
            markdown: None,
            memory: None,
            archive: None,
            hosting: None,
            out_dir: None,
        }
    }
}

/// LRU budgets for the memory tier.
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryConfig {
    /// Maximum total bytes of raw content kept in memory (`0` =
    /// unlimited). When the budget is exceeded, the least recently used
    /// entries are evicted (they remain readable from the disk archive).
    pub byte_budget: Option<i64>,
    /// Maximum number of entries in memory (`0` = unlimited).
    pub entry_budget: Option<u32>,
}

impl MemoryConfig {
    /// Resolved byte budget (`0` = unlimited).
    pub fn byte_budget(&self) -> u64 {
        self.byte_budget
            .unwrap_or(DEFAULT_BYTE_BUDGET as i64)
            .max(0) as u64
    }

    /// Resolved entry budget (`0` = unlimited).
    pub fn entry_budget(&self) -> usize {
        self.entry_budget.unwrap_or(0) as usize
    }
}

/// Flush thresholds for the disk-archive tier.
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Clone, Copy, Default)]
pub struct ArchiveConfig {
    /// Pending-content threshold (bytes) that triggers writing a new
    /// shard; `0` disables size-based flushing (interval/drop only).
    pub shard_bytes: Option<i64>,
    /// Maximum time (ms) pending entries may sit in the worker buffer
    /// before being flushed. `0` disables the timer.
    pub flush_interval_ms: Option<i64>,
}

impl ArchiveConfig {
    /// Resolved shard threshold.
    pub fn shard_bytes(&self) -> u64 {
        self.shard_bytes
            .unwrap_or(DEFAULT_SHARD_BYTES as i64)
            .max(0) as u64
    }

    /// Resolved flush interval.
    pub fn flush_interval(&self) -> Duration {
        Duration::from_millis(
            self.flush_interval_ms
                .unwrap_or(DEFAULT_FLUSH_INTERVAL_MS as i64)
                .max(0) as u64,
        )
    }
}

/// Static-hosting configuration for baseUrl resolution.
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Clone, Default)]
pub struct HostingConfig {
    /// Hosting provider (see [`HostingProvider`]).
    pub provider: Option<HostingProvider>,
    /// GitHub repository name for project pages (`/<repo>/` prefix).
    pub repo: Option<String>,
    /// Custom site URL (overrides provider defaults when set).
    pub site_url: Option<String>,
    /// Extra path prefix (e.g. `"subpath"` for `example.com/subpath/`).
    pub path_prefix: Option<String>,
}

impl HostingConfig {
    fn resolve(&self) -> BaseDirInfo {
        let mut info = BaseDirInfo::new(
            self.provider.unwrap_or(HostingProvider::Custom),
            self.repo.clone(),
            self.site_url.clone(),
        );
        info.path_prefix = self.path_prefix.clone();
        info
    }
}

impl Default for BaseDirInfo {
    fn default() -> Self {
        Self::new(HostingProvider::Custom, None, None)
    }
}

/// Default LRU byte budget when none is configured: 64 MiB.
pub const DEFAULT_BYTE_BUDGET: u64 = 64 * 1024 * 1024;
/// Default pending-content shard threshold: 8 MiB.
pub const DEFAULT_SHARD_BYTES: u64 = 8 * 1024 * 1024;
/// Default archive flush interval: 2 seconds.
pub const DEFAULT_FLUSH_INTERVAL_MS: u64 = 2_000;
/// Default build output directory.
pub const DEFAULT_OUT_DIR: &str = ".peisar";

/// Boxed change callback held in the subscription registry.
///
/// In npm builds, JavaScript callbacks are wrapped into a thread-safe
/// function before they are stored because watcher events fire off-thread.
pub type CacheChangeCallback = Box<dyn Fn(CacheChangeEvent) + Send + 'static>;

/// Backward-compatible name for callbacks used by the JavaScript binding.
pub type JsChangeCallback = CacheChangeCallback;

/// Shared registry of change callbacks, keyed by subscription id.
#[derive(Clone, Default)]
struct ChangeCallbacks {
    inner: Arc<Mutex<HashMap<u32, CacheChangeCallback>>>,
    next_id: Arc<AtomicU32>,
}

impl ChangeCallbacks {
    fn add(&self, cb: CacheChangeCallback) -> u32 {
        let mut map = self.inner.lock().unwrap();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        map.insert(id, cb);
        id
    }

    fn remove(&self, id: u32) {
        self.inner.lock().unwrap().remove(&id);
    }

    fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }

    fn dispatch(&self, event: CacheChangeEvent) {
        let map = self.inner.lock().unwrap();
        for cb in map.values() {
            cb(event.clone());
        }
    }
}

/// In-memory + archive-backed cache of the Markdown and asset files under a
/// directory tree.
///
/// Memory is the source of truth for hot content; a background worker
/// flushes a binary shard archive to `.peisar-cache` for cold/large
/// entries, front-ended by the `map.json` search index. Watchers keep the
/// memory tier in sync with the filesystem.
#[cfg_attr(feature = "npm", napi)]
pub struct PeisarCache {
    /// LRU memory tier: absolute source path → raw content. Clone-shared
    /// with the watcher thread.
    memory: LruCache,
    /// Disk-archive tier handle (shards + map.json index). Clone-shared
    /// with the watcher thread.
    archive: ArchiveHandle,
    /// Registered visitors (JS or Rust adapters), applied to every render.
    visitors: Arc<Mutex<Vec<RegisteredVisitor>>>,
    /// Registered parser hooks, applied to every render.
    parsers: Arc<Mutex<Vec<RegisteredParser>>>,
    /// Resolved markdown AST options (GFM/Kramdown, file name).
    ast_opts: AstOptions,
    /// Resolved markdown render options (fragment, title, …).
    render_opts: RenderOptions,
    watcher: Option<RecommendedWatcher>,
    entry_dir: PathBuf,
    assets_dir: Option<PathBuf>,
    change_callbacks: ChangeCallbacks,
    /// Hosting/baseUrl configuration.
    hosting: BaseDirInfo,
    /// Build output directory (for `outFilePath` fields).
    out_dir: String,
}

/// Render one markdown document into HTML using the resolved options and
/// the registered visitors/parsers (if any).
fn render_markdown(
    raw_md: &str,
    ast_opts: &AstOptions,
    render_opts: &RenderOptions,
    visitors: &Arc<Mutex<Vec<RegisteredVisitor>>>,
    parsers: &Arc<Mutex<Vec<RegisteredParser>>>,
    file_name: Option<String>,
) -> String {
    // Front matter never reaches the renderer.
    let (md_body, _fm) = match frontmatter(raw_md.to_string()) {
        Ok(parsed) => parsed.into_parts(),
        Err(_) => (raw_md.to_string(), None),
    };
    let hooks_lock = parsers.lock().unwrap();
    let mut hooks = ParseHooks::empty();
    for p in hooks_lock.iter() {
        hooks.push(p);
    }
    let opts = AstOptions {
        gfm: ast_opts.gfm,
        kramdown: ast_opts.kramdown,
        file_name,
    };
    let mut doc = md_to_ast_with_hooks(&md_body, &opts, opts.file_name.clone(), &hooks);
    let mut visitors_lock = visitors.lock().unwrap();
    for v in visitors_lock.iter_mut() {
        visit_document_mut(&mut doc, v);
    }
    render_document_html(&doc, Some(render_opts.clone()))
}

/// Extract a page title: front matter `title` first, else the first `#`
/// heading before any other block content.
fn extract_title(fm: Option<&serde_json::Value>, md_body: &str) -> Option<String> {
    if let Some(title) = fm.and_then(|fm| fm.get("title")).and_then(|t| t.as_str()) {
        return Some(title.to_string());
    }
    for line in md_body.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            return Some(rest.trim().to_string());
        } else if !trimmed.is_empty() {
            // First non-empty, non-heading line ends the search.
            return None;
        }
    }
    None
}

/// Front-matter field helpers (string / string-array flattening).
fn fm_string(fm: Option<&serde_json::Value>, keys: &[&str]) -> Option<String> {
    let fm = fm?;
    for key in keys {
        if let Some(v) = fm.get(*key) {
            return Some(match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            });
        }
    }
    None
}

fn fm_string_list(fm: Option<&serde_json::Value>, key: &str) -> Option<Vec<String>> {
    let fm = fm?;
    let arr = fm.get(key)?.as_array()?;
    Some(
        arr.iter()
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect(),
    )
}

/// Base64 encode bytes (standard alphabet with padding) without a
/// dependency — used for `AssetsObject.raw` of binary assets.
fn binary_to_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 63) as usize] as char);
        out.push(TABLE[((triple >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg_attr(feature = "npm", napi)]
impl PeisarCache {
    /// JS: `new PeisarCache(entryDir, assetsDir?, options?)`.
    ///
    /// `options` configures markdown parsing (`fragment`, GFM, …), archive
    /// thresholds, hosting resolution, and the output directory. Custom
    /// visitors/parsers register afterwards via `useVisitor` /
    /// `useParser`.
    #[cfg(feature = "npm")]
    #[napi(constructor)]
    pub fn new_js(
        entry_dir: String,
        assets_dir: Option<String>,
        options: Option<PeisarCacheJsOptions>,
    ) -> Result<Self> {
        let mut cache = PeisarCache::with_config(PeisarCacheConfig {
            entry_dir,
            assets_dir,
            markdown: options.as_ref().and_then(|o| o.markdown.clone()),
            archive: options.as_ref().and_then(|o| o.archive),
            hosting: options.as_ref().and_then(|o| o.hosting.clone()),
            out_dir: options.as_ref().and_then(|o| o.out_dir.clone()),
            memory: None,
        })
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, format!("{e}")))?;
        let _ = &mut cache; // silence unused-assignment lint paths
        Ok(cache)
    }

    /// JS: `PeisarCache.withConfigJs(config)` — full configuration form,
    /// including memory-tier budgets.
    #[cfg(feature = "npm")]
    #[napi(factory)]
    pub fn with_config_js(config: PeisarCacheConfig) -> Result<Self> {
        PeisarCache::with_config(config)
            .map_err(|e| napi::Error::new(napi::Status::GenericFailure, format!("{e}")))
    }

    /// Construct a cache from a full configuration (Rust entry point).
    ///
    /// Construction loads all markdown (and optionally assets) into the
    /// memory tier; disk writes happen later on the archive worker's
    /// size/interval flush (or explicitly via [`Self::flush`]).
    pub fn with_config(config: PeisarCacheConfig) -> io::Result<Self> {
        let cwd = env::current_dir()?;
        // Resolve the entry dir to an absolute path: watcher events carry
        // absolute paths, so the `is_markdown` prefix check needs an
        // absolute entry_dir even when JS passes a relative one.
        let entry_dir = {
            let d = Path::new(&config.entry_dir);
            if d.is_absolute() {
                d.to_path_buf()
            } else {
                cwd.join(d)
            }
        };

        // Assets default to `public` under the cwd when omitted.
        let resolved_assets: Option<PathBuf> = match &config.assets_dir {
            Some(p) => {
                let pbuf = PathBuf::from(p);
                Some(if pbuf.is_absolute() {
                    pbuf
                } else {
                    cwd.join(pbuf)
                })
            }
            None => {
                let p = cwd.join("public");
                if p.exists() && p.is_dir() {
                    Some(p)
                } else {
                    None
                }
            }
        };

        // Markdown options for every page render.
        let resolved = get_options(config.markdown.clone());
        let ast_opts = resolved.ast_opts;
        let render_opts = resolved.render_opts;

        // Memory tier.
        let mem_cfg = config.memory.unwrap_or_default();
        let memory = LruCache::new(mem_cfg.byte_budget(), mem_cfg.entry_budget());

        // Archive tier.
        let archive_cfg = config.archive.unwrap_or_default();
        let archive = ArchiveHandle::new(
            cwd.clone(),
            archive_cfg.shard_bytes(),
            archive_cfg.flush_interval(),
        );

        // Hosting resolution.
        let hosting = config
            .hosting
            .as_ref()
            .map(HostingConfig::resolve)
            .unwrap_or_default();

        // Discover and load markdown + assets into the memory tier.
        let mut cache_map: HashMap<PathBuf, CachedContent> = HashMap::new();
        for f in collect_markdown_files(&entry_dir)? {
            if let Ok(text) = fs::read_to_string(&f) {
                cache_map.insert(f, CachedContent::Text(text));
            }
        }
        if let Some(ref a_dir) = resolved_assets
            && let Ok(asset_files) = collect_asset_files(a_dir)
        {
            for f in asset_files {
                let is_binary = f
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| ASSET_BINARY_EXTENSIONS.contains(&s.to_lowercase().as_str()))
                    .unwrap_or(false);
                if is_binary {
                    if let Ok(bytes) = fs::read(&f) {
                        cache_map.insert(f, CachedContent::Binary(bytes));
                    }
                } else if let Ok(text) = fs::read_to_string(&f) {
                    cache_map.insert(f, CachedContent::Text(text));
                }
            }
        }
        memory.replace_all(cache_map.clone());

        // Enqueue the initial snapshot to the archive tier (flushed by
        // size/interval later — no blocking disk writes at construction).
        archive.sync_all(cache_map);

        // Make sure the archive dir is ignored by git.
        archive::ensure_gitignore_entry(&cwd)?;

        // The visitor/parser registries are intentionally non-Send:
        // they capture the napi `Env`, which must not cross threads.
        // Rendering (the only code that touches them) runs on the JS main
        // thread; the watcher thread shares only the `Send + Sync`
        // `LruCache` / `ArchiveHandle`.
        #[allow(clippy::arc_with_non_send_sync)]
        let visitors: Arc<Mutex<Vec<RegisteredVisitor>>> = Arc::new(Mutex::new(Vec::new()));
        #[allow(clippy::arc_with_non_send_sync)]
        let parsers: Arc<Mutex<Vec<RegisteredParser>>> = Arc::new(Mutex::new(Vec::new()));
        Ok(PeisarCache {
            memory,
            archive,
            visitors,
            parsers,
            ast_opts,
            render_opts,
            watcher: None,
            entry_dir,
            assets_dir: resolved_assets,
            change_callbacks: ChangeCallbacks::default(),
            hosting,
            out_dir: config
                .out_dir
                .unwrap_or_else(|| DEFAULT_OUT_DIR.to_string()),
        })
    }

    /// Construct a [`PeisarCache`] over `entry_dir` with the default
    /// configuration — Rust convenience (generic signatures cannot cross
    /// the NAPI boundary).
    pub fn new<P: AsRef<Path>>(entry_dir: P) -> io::Result<Self> {
        PeisarCache::with_config(PeisarCacheConfig {
            entry_dir: entry_dir.as_ref().to_string_lossy().to_string(),
            ..Default::default()
        })
    }

    /// Register a JavaScript AST visitor applied to every page render
    /// (JS: `cache.useVisitor({ visitBlock, visitInline })`).
    #[cfg(feature = "npm")]
    #[napi]
    pub fn use_visitor(&mut self, env: napi::Env, visitor: Visitor) {
        self.visitors.lock().unwrap().push(visitor.register(env));
    }

    /// Register a JavaScript parser hook applied to every page render
    /// (JS: `cache.useParser({ parseBlock, parseInline })`).
    #[cfg(feature = "npm")]
    #[napi]
    pub fn use_parser(&mut self, env: napi::Env, parser: Parser) {
        self.parsers.lock().unwrap().push(parser.register(env));
    }

    /// JS: `cache.startWatchingJs()` — start watching the entry (and
    /// assets) directories recursively; errors surface as JS exceptions.
    #[cfg(feature = "npm")]
    #[napi]
    pub fn start_watching_js(&mut self) -> Result<()> {
        self.start_watching()
            .map_err(|e| napi::Error::new(napi::Status::GenericFailure, format!("{e}")))
    }

    /// Start watching the entry (and assets) directories recursively.
    ///
    /// From then on, create/modify/remove events update the memory tier
    /// first, then enqueue matching archive commands; directory events
    /// trigger a full rescan. Every operation also logs one pretty line
    /// to stderr — `created contents/docs/intro.md` — with the path
    /// relative to the project root. The label is derived from the
    /// actual filesystem state (editors surface atomic saves as rename
    /// events, so the raw event kind cannot be trusted), duplicate events
    /// for one save coalesce (an edit logs `modified` exactly once),
    /// and filesystem noise is not logged; `onChange` subscribers still
    /// receive every raw event (see [`PeisarCache::on_change`]).
    ///
    /// The returned [`io::Result`] covers watcher setup errors only; runtime
    /// errors are printed to stderr by the watch callback.
    pub fn start_watching(&mut self) -> io::Result<()> {
        // The watcher thread shares the memory tier and archive handle
        // through clone-shared Arcs.
        let memory = self.memory.clone();
        let archive = self.archive.clone();
        let entry_dir = self.entry_dir.clone();
        let assets_dir = self.assets_dir.clone();
        let change_callbacks = self.change_callbacks.clone();
        // Pretty logs render paths relative to the project root (the
        // cwd the cache was constructed in).
        let log_root = env::current_dir()?;
        // One log line per file operation: remember when each path last
        // logged so duplicate events from a single save (truncate +
        // write, atomic rename, …) coalesce into one line.
        let mut last_log_at: HashMap<PathBuf, Instant> = HashMap::new();

        let mut watcher: RecommendedWatcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                match res {
                    Ok(event) => {
                        for path in event.paths {
                            let is_markdown = path.starts_with(&entry_dir);
                            // The notify event kind cannot be trusted for
                            // the correct label (editors and `mv` surface
                            // atomic saves as rename/"modify" events), so
                            // classify from the actual filesystem state.
                            let kind_str = classify_change(&memory, &archive, &path);
                            change_callbacks.dispatch(CacheChangeEvent {
                                path: path.to_string_lossy().to_string(),
                                kind: kind_str.to_string(),
                                is_markdown,
                            });
                            // One log line per file operation, and only
                            // for files the cache could ever track: the
                            // same path may fire several events (rename
                            // pairs, atomic saves) and editor side files
                            // (`.tmp`, swap) never log.
                            let now = Instant::now();
                            if should_log_change(&path, &last_log_at, now) {
                                log_change_pretty(kind_str, &path, &log_root);
                                last_log_at.insert(path.clone(), now);
                            }
                            // Directory events rescan everything.
                            if path.is_dir() {
                                let mut new_map: HashMap<PathBuf, CachedContent> = HashMap::new();
                                if let Ok(files) = collect_markdown_files(&entry_dir) {
                                    for f in files {
                                        if let Ok(text) = fs::read_to_string(&f) {
                                            new_map.insert(f, CachedContent::Text(text));
                                        }
                                    }
                                }
                                if let Some(ref a_dir) = assets_dir
                                    && let Ok(asset_files) = collect_asset_files(a_dir)
                                {
                                    for f in asset_files {
                                        let ext_l = f
                                            .extension()
                                            .and_then(|e| e.to_str())
                                            .map(|s| s.to_lowercase())
                                            .unwrap_or_default();
                                        if ASSET_EXTENSIONS.contains(&ext_l.as_str()) {
                                            let is_binary =
                                                ASSET_BINARY_EXTENSIONS.contains(&ext_l.as_str());
                                            if is_binary {
                                                if let Ok(bytes) = fs::read(&f) {
                                                    new_map.insert(f, CachedContent::Binary(bytes));
                                                }
                                            } else if let Ok(text) = fs::read_to_string(&f) {
                                                new_map.insert(f, CachedContent::Text(text));
                                            }
                                        }
                                    }
                                }
                                memory.replace_all(new_map.clone());
                                archive.sync_all(new_map);
                                continue;
                            }

                            if path.is_file()
                                && let Some(ext) = path.extension().and_then(|e| e.to_str())
                            {
                                let ext_l = ext.to_lowercase();
                                if MARKDOWN_EXTENSIONS.contains(&ext_l.as_str())
                                    || ASSET_EXTENSIONS.contains(&ext_l.as_str())
                                {
                                    let is_binary =
                                        ASSET_BINARY_EXTENSIONS.contains(&ext_l.as_str());
                                    let content = if is_binary {
                                        fs::read(&path).ok().map(CachedContent::Binary)
                                    } else {
                                        fs::read_to_string(&path).ok().map(CachedContent::Text)
                                    };
                                    match content {
                                        Some(content) => {
                                            memory.insert(path.clone(), content.clone());
                                            archive.upsert(path, content);
                                        }
                                        None => {
                                            memory.remove(&path);
                                            archive.remove(path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("watch error: {e:?}"),
                }
            })
            .map_err(|e| io::Error::other(format!("notify::recommended_watcher error: {e}")))?;

        watcher
            .watch(&self.entry_dir, RecursiveMode::Recursive)
            .map_err(|e| io::Error::other(format!("notify::watch error: {e}")))?;

        if let Some(a) = &self.assets_dir {
            watcher
                .watch(a, RecursiveMode::Recursive)
                .map_err(|e| io::Error::other(format!("notify::watch error: {e}")))?;
        }

        self.watcher = Some(watcher);
        Ok(())
    }

    /// Stop watching the entry/assets directories and flush pending
    /// archive writes. The Rust counterpart of the JS `dispose()`: use it
    /// to shut the cache down gracefully — e.g. on `Ctrl+C` — instead of
    /// relying on the automatic [`Drop`] cleanup. Safe to call more than
    /// once (and without ever calling `start_watching`).
    pub fn stop_watching(&mut self) {
        self.watcher = None;
        self.archive.flush();
    }

    /// Read one entry: memory tier first, then the disk archive. Returns
    /// `None` when the path is unknown to both tiers.
    pub fn get(&self, path: &Path) -> Option<CachedContent> {
        if let Some(content) = self.memory.get(path) {
            return Some(content);
        }
        self.archive.read(path)
    }

    /// Clone of every cached markdown entry: absolute path → raw text.
    pub fn all(&self) -> HashMap<PathBuf, String> {
        self.memory
            .iter_all()
            .into_iter()
            .filter_map(|(p, c)| match c {
                CachedContent::Text(s) => Some((p, s)),
                _ => None,
            })
            .collect()
    }

    /// Union of the memory-tier and archive-tier keys.
    fn all_keys(&self) -> HashSet<PathBuf> {
        let mut keys: HashSet<PathBuf> = HashSet::new();
        for k in self.memory.keys() {
            keys.insert(k);
        }
        for k in self.archive.keys() {
            keys.insert(k);
        }
        keys
    }

    /// JS: `cache.getText(absPath)` — cached text of a file, or `null`.
    /// Reads fall through to the archive tier for evicted entries.
    #[cfg_attr(feature = "npm", napi)]
    pub fn get_text(&self, abs_path: String) -> Option<String> {
        self.get(Path::new(&abs_path)).and_then(|c| match c {
            CachedContent::Text(s) => Some(s),
            _ => None,
        })
    }

    /// JS: `cache.getBinary(absPath)` — cached bytes of a binary asset, or
    /// `null`. Reads fall through to the archive tier.
    #[cfg_attr(feature = "npm", napi)]
    pub fn get_binary(&self, abs_path: String) -> Option<Vec<u8>> {
        self.get(Path::new(&abs_path)).and_then(|c| match c {
            CachedContent::Binary(b) => Some(b),
            _ => None,
        })
    }

    /// JS: `cache.listFiles()` — absolute paths of everything cached
    /// (memory + archive tiers).
    #[cfg_attr(feature = "npm", napi)]
    pub fn list_files(&self) -> Vec<String> {
        self.all_keys()
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }

    /// JS: `cache.markdownFiles()` — absolute paths of cached markdown
    /// files (either tier).
    #[cfg_attr(feature = "npm", napi)]
    pub fn markdown_files(&self) -> Vec<String> {
        self.filter_cached_files(|ext| MARKDOWN_EXTENSIONS.contains(&ext))
    }

    /// JS: `cache.assetFiles()` — absolute paths of cached non-markdown
    /// files (either tier).
    #[cfg_attr(feature = "npm", napi)]
    pub fn asset_files(&self) -> Vec<String> {
        self.filter_cached_files(|ext| !MARKDOWN_EXTENSIONS.contains(&ext))
    }

    /// Filter the union of memory + archive keys by extension predicate.
    fn filter_cached_files(&self, keep: fn(&str) -> bool) -> Vec<String> {
        self.all_keys()
            .into_iter()
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(keep)
                    .unwrap_or(false)
            })
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }
    // ===== Derived page / asset objects (Liquid template shapes) =====

    /// Rust: every page object, sorted by slug.
    pub fn page_objects(&self) -> Vec<PageObject> {
        let mut pages: Vec<PageObject> = Vec::new();
        for path in self.all_keys() {
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| MARKDOWN_EXTENSIONS.contains(&e))
                .unwrap_or(false)
                && let Some(page) = self.page_object_for(&path)
            {
                pages.push(page);
            }
        }
        pages.sort_by(|a, b| a.slug.cmp(&b.slug));
        pages
    }

    /// JS: `cache.pages()` — every page object, for Liquid themes
    /// (`{{ page.title }}`, `{{ page.layout }}`, …).
    #[cfg_attr(feature = "npm", napi)]
    pub fn pages(&self) -> Vec<PageObject> {
        self.page_objects()
    }

    /// Build one [`PageObject`] for a markdown path (memory or archive
    /// tier). Returns `None` when the entry is unknown or unreadable.
    pub fn page_object_for(&self, path: &Path) -> Option<PageObject> {
        let content = self.get(path)?;
        let raw_md = match content {
            CachedContent::Text(s) => s,
            _ => return None,
        };
        let (md_body, fm) = match frontmatter(raw_md.to_string()) {
            Ok(parsed) => parsed.into_parts(),
            Err(_) => (raw_md.clone(), None),
        };
        let fm_ref = fm.as_ref();
        let layout = fm_string(fm_ref, &["layout"]).unwrap_or_else(|| "default".to_string());
        let title = extract_title(fm_ref, &md_body);
        let fname = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let file_name = file_name_no_ext(&fname);
        let base_url = self.hosting.base_url();
        Some(PageObject {
            layout,
            file_name: file_name.clone(),
            slug: markdown_slug(path, &self.entry_dir, &base_url),
            html: render_markdown(
                &raw_md,
                &self.ast_opts,
                &self.render_opts,
                &self.visitors,
                &self.parsers,
                Some(file_name),
            ),
            out_file_path: page_out_file(path, &self.entry_dir, &self.out_dir),
            input_file_path: path.to_string_lossy().to_string(),
            title,
            summary: fm_string(fm_ref, &["summary", "description"]),
            tags: fm_string_list(fm_ref, "tags"),
            publish_date: fm_string(fm_ref, &["date", "publishDate", "publish_date"]),
            extra: fm_ref.and_then(extra_frontmatter_fields),
        })
    }

    /// JS: `cache.getPage(inputPath)` — a single page object by absolute
    /// input path, or `null`.
    #[cfg_attr(feature = "npm", napi)]
    pub fn get_page(&self, input_path: String) -> Option<PageObject> {
        self.page_object_for(Path::new(&input_path))
    }

    /// Rust: every asset object, sorted by slug.
    pub fn asset_objects(&self) -> Vec<AssetsObject> {
        let mut assets: Vec<AssetsObject> = Vec::new();
        for path in self.all_keys() {
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| !MARKDOWN_EXTENSIONS.contains(&e))
                .unwrap_or(false)
                && let Some(asset) = self.asset_object_for(&path)
            {
                assets.push(asset);
            }
        }
        assets.sort_by(|a, b| a.slug.cmp(&b.slug));
        assets
    }

    /// JS: `cache.assets()` — every asset object.
    #[cfg_attr(feature = "npm", napi)]
    pub fn assets(&self) -> Vec<AssetsObject> {
        self.asset_objects()
    }

    /// Build one [`AssetsObject`] for an asset path (either tier).
    pub fn asset_object_for(&self, path: &Path) -> Option<AssetsObject> {
        let content = self.get(path)?;
        let base_dir = self.assets_dir.as_deref().unwrap_or(&self.entry_dir);
        let fname = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        let raw = match &content {
            CachedContent::Text(s) => s.clone(),
            CachedContent::Binary(b) => binary_to_base64(b),
        };
        let base_url = self.hosting.base_url();
        Some(AssetsObject {
            file_name: file_name_no_ext(&fname),
            slug: asset_slug(path, base_dir, &base_url),
            raw,
            out_file_path: asset_out_file(path, base_dir, &self.out_dir),
            input_file_path: path.to_string_lossy().to_string(),
            ext: ext.clone(),
            file_type: asset_file_type(&ext).to_string(),
        })
    }

    /// JS: `cache.getAsset(inputPath)` — a single asset object by absolute
    /// input path, or `null`.
    #[cfg_attr(feature = "npm", napi)]
    pub fn get_asset(&self, input_path: String) -> Option<AssetsObject> {
        self.asset_object_for(Path::new(&input_path))
    }

    // ===== Archive-tier introspection =====

    /// JS: `cache.archived()` — absolute paths flushed to the disk
    /// archive. Everything not listed still lives in memory only.
    #[cfg_attr(feature = "npm", napi)]
    pub fn archived(&self) -> Vec<String> {
        self.archive
            .keys()
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }

    /// JS: `cache.mapJson()` — the current `map.json` document (path →
    /// shard location + metadata, including front matter for search).
    #[cfg_attr(feature = "npm", napi)]
    pub fn map_json(&self) -> String {
        self.archive.map_json()
    }

    /// JS: `cache.flush()` — write all pending archive entries now,
    /// blocking until the worker is done.
    #[cfg_attr(feature = "npm", napi)]
    pub fn flush(&self) {
        self.archive.flush();
    }

    // ===== Hosting / baseUrl =====

    /// JS: `cache.baseUrl()` — the resolved base URL path prefix for the
    /// configured hosting (e.g. `/repo/` for a GitHub Pages project site).
    #[cfg_attr(feature = "npm", napi)]
    pub fn base_url(&self) -> String {
        self.hosting.base_url()
    }

    /// JS: `cache.resolveUrl(path)` — prefix a site-relative path with the
    /// resolved base URL: `resolveUrl("docs/intro")` → `/repo/docs/intro`.
    #[cfg_attr(feature = "npm", napi)]
    pub fn resolve_url(&self, path: String) -> String {
        let base = self.hosting.base_url();
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            base
        } else {
            format!("{}/{}", base.trim_end_matches('/'), path)
        }
    }

    /// JS: `cache.siteUrl()` — absolute origin URL for canonical URLs, or
    /// `null` when not configured.
    #[cfg_attr(feature = "npm", napi)]
    pub fn site_url(&self) -> Option<String> {
        self.hosting.site_url()
    }

    // ===== Subscriptions and disposal =====

    /// JS: `cache.onChange(cb)` — invoke `cb(event)` on every file change the
    /// watcher detects while watching is active. Returns a subscription id
    /// that can be passed to `offChange(id)`.
    ///
    /// The callback is wrapped in a `ThreadsafeFunction` because notify
    /// events fire on the watcher thread, not the JS main thread.
    #[cfg(feature = "npm")]
    #[napi]
    pub fn on_change(
        &self,
        #[napi(ts_arg_type = "(event: CacheChangeEvent) => void")] callback: Function<
            'static,
            CacheChangeEvent,
            Unknown<'static>,
        >,
    ) -> Result<u32> {
        use napi::threadsafe_function::ThreadsafeFunctionCallMode;

        // Weak TSFN: the subscription must not keep the Node event loop
        // alive on its own — long-running consumers (dev server) keep the
        // loop alive via their own handles (http listener), so an
        // onChange-only script should exit when its work is done.
        let tsfn = callback
            .build_threadsafe_function::<CacheChangeEvent>()
            .weak::<true>()
            .build()?;

        Ok(self
            .change_callbacks
            .add(Box::new(move |event: CacheChangeEvent| {
                tsfn.call(event, ThreadsafeFunctionCallMode::NonBlocking);
            })))
    }

    /// Register a Rust callback that receives every change detected while
    /// watching is active. Returns a subscription id that can be passed to
    /// [`PeisarCache::off_change`].
    #[cfg(not(feature = "npm"))]
    pub fn on_change<F>(&self, callback: F) -> u32
    where
        F: Fn(CacheChangeEvent) + Send + 'static,
    {
        self.change_callbacks.add(Box::new(callback))
    }

    /// JS: `cache.offChange(id)` — remove a previously registered callback.
    #[cfg_attr(feature = "npm", napi)]
    pub fn off_change(&self, id: u32) {
        self.change_callbacks.remove(id);
    }

    /// JS: `cache.dispose()` — stop the watcher, flush pending archive
    /// writes, and drop JS change callbacks. Safe to call more than once.
    ///
    /// Rust consumers do not need this: dropping the [`PeisarCache`] stops
    /// the watcher, flushes the archive, and joins the workers.
    #[cfg_attr(feature = "npm", napi)]
    pub fn dispose(&mut self) {
        self.stop_watching();
        self.change_callbacks.clear();
    }
}

/// JS-side options accepted by the `PeisarCache` constructor (a flattened
/// subset of [`PeisarCacheConfig`]; memory budgets only apply through the
/// `withConfig` factory).
#[cfg(feature = "npm")]
#[napi(object)]
pub struct PeisarCacheJsOptions {
    /// Markdown parsing/rendering options (`fragment`, GFM, …).
    pub markdown: Option<PeisarOptions>,
    /// Disk-archive thresholds (shard size / flush interval).
    pub archive: Option<ArchiveConfig>,
    /// Static-hosting baseUrl resolution.
    pub hosting: Option<HostingConfig>,
    /// Build output directory.
    pub out_dir: Option<String>,
}

// Dropping a `PeisarCache` is the Rust equivalent of the JS `dispose()`:
// the watcher is released with the struct, the archive worker flushes its
// pending buffer and exits, and the memory tier goes away with it.
impl Drop for PeisarCache {
    fn drop(&mut self) {
        self.watcher = None;
        self.change_callbacks.clear();
        self.archive.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    static TEST_CWD_LOCK: Mutex<()> = Mutex::new(());

    fn make_tmp(tag: &str) -> io::Result<(PathBuf, PathBuf)> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| io::Error::other(format!("time error: {e}")))?
            .as_nanos();
        let tmp = env::temp_dir().join(format!("peisar_test_{tag}_{now}"));
        if tmp.exists() {
            fs::remove_dir_all(&tmp)?;
        }
        fs::create_dir_all(&tmp)?;
        let orig = env::current_dir()?;
        Ok((tmp, orig))
    }

    fn write_md(dir: &Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = File::create(p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.sync_all().unwrap();
    }

    fn restore(orig: PathBuf, tmp: PathBuf) {
        let _ = env::set_current_dir(orig);
        let _ = fs::remove_dir_all(tmp);
    }

    #[cfg(not(feature = "npm"))]
    #[test]
    fn on_change_registers_a_rust_callback() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let (tmp, orig) = make_tmp("change")?;
        env::set_current_dir(&tmp)?;

        let cache = PeisarCache::new(&tmp)?;
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_callback = Arc::clone(&received);
        let subscription = cache.on_change(move |event| {
            received_callback.lock().unwrap().push(event);
        });

        cache.change_callbacks.dispatch(CacheChangeEvent {
            path: "/tmp/example.md".to_string(),
            kind: "modify".to_string(),
            is_markdown: true,
        });
        cache.off_change(subscription);

        assert_eq!(received.lock().unwrap().len(), 1);
        restore(orig, tmp);
        Ok(())
    }

    #[test]
    fn memory_first_no_disk_at_construction() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let (tmp, orig) = make_tmp("memfirst")?;
        env::set_current_dir(&tmp)?;
        write_md(&tmp, "docs/doc1.md", "---\ntitle: Test\n---\n# Hello\n");

        let cache = PeisarCache::new("docs")?;

        // Memory tier has the content immediately.
        let all = cache.all();
        assert_eq!(all.len(), 1);
        let content = cache.get(&tmp.join("docs").join("doc1.md")).unwrap();
        assert!(matches!(content, CachedContent::Text(ref s) if s.contains("# Hello")));

        // Nothing flushed yet: the archive worker only flushes on
        // size/interval/drop — and the default interval is 2 s.
        assert_eq!(cache.archived().len(), 0);
        let archive_dir = tmp.join(ARCHIVE_DIR_NAME);
        assert!(!archive_dir.join("map.json").exists());

        restore(orig, tmp);
        Ok(())
    }

    #[test]
    fn flush_writes_map_and_shards() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let (tmp, orig) = make_tmp("flush")?;
        env::set_current_dir(&tmp)?;
        write_md(&tmp, "docs/a.md", "# A\n");

        let cache = PeisarCache::new("docs")?;
        cache.flush();

        let archive_dir = tmp.join(ARCHIVE_DIR_NAME);
        let map_str = fs::read_to_string(archive_dir.join("map.json"))?;
        let map: serde_json::Value = serde_json::from_str(&map_str).unwrap();
        assert_eq!(map["version"], 1);
        assert!(map["files"]["docs/a.md"]["frontmatter"].is_null());

        assert_eq!(cache.archived().len(), 1);
        assert!(archive_dir.join("shard-000001.bin").exists());

        restore(orig, tmp);
        Ok(())
    }

    #[test]
    fn page_objects_liquid_shape() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let (tmp, orig) = make_tmp("pages")?;
        env::set_current_dir(&tmp)?;
        write_md(
            &tmp,
            "contents/index.md",
            "---\ntitle: Home Page\nlayout: custom\nsummary: hi there\ntags:\n  - a\n  - b\ndate: 2026-01-01\nauthor: someone\n---\n# Not the title\n\nBody\n",
        );
        write_md(&tmp, "contents/docs/intro.md", "# Intro\n");

        let cache = PeisarCache::with_config(PeisarCacheConfig {
            entry_dir: "contents".into(),
            markdown: Some(PeisarOptions {
                fragment: Some(true),
                ..Default::default()
            }),
            hosting: Some(HostingConfig {
                provider: Some(HostingProvider::GithubPages),
                repo: Some("my-repo".into()),
                ..Default::default()
            }),
            out_dir: Some("dist".into()),
            ..Default::default()
        })?;

        let pages = cache.page_objects();
        assert_eq!(pages.len(), 2);

        let home = pages
            .iter()
            .find(|p| p.input_file_path.ends_with("index.md"))
            .unwrap();
        assert_eq!(home.layout, "custom");
        assert_eq!(home.title.as_deref(), Some("Home Page"));
        assert_eq!(home.summary.as_deref(), Some("hi there"));
        assert_eq!(home.tags, Some(vec!["a".to_string(), "b".to_string()]));
        assert_eq!(home.publish_date.as_deref(), Some("2026-01-01"));
        // extra fields are flattened and stringified
        assert_eq!(home.extra.as_ref().unwrap()["author"], "someone");
        // baseUrl applied
        assert_eq!(home.slug, "/my-repo/");
        assert_eq!(home.out_file_path, "dist/index.html");
        assert!(home.html.contains("Body"));

        let intro = pages
            .iter()
            .find(|p| p.input_file_path.ends_with("intro.md"))
            .unwrap();
        // Title falls back to the first heading
        assert_eq!(intro.title.as_deref(), Some("Intro"));
        assert_eq!(intro.slug, "/my-repo/docs/intro");
        assert_eq!(intro.out_file_path, "dist/docs/intro/index.html");
        // layout defaults
        assert_eq!(intro.layout, "default");

        restore(orig, tmp);
        Ok(())
    }

    #[test]
    fn asset_objects_shape() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let (tmp, orig) = make_tmp("assets")?;
        env::set_current_dir(&tmp)?;
        write_md(&tmp, "contents/index.md", "# Home\n");
        fs::create_dir_all(tmp.join("public").join("img"))?;
        fs::write(
            tmp.join("public").join("img").join("logo.png"),
            vec![1u8, 2, 3],
        )?;

        let cache = PeisarCache::with_config(PeisarCacheConfig {
            entry_dir: "contents".into(),
            assets_dir: Some("public".into()),
            ..Default::default()
        })?;

        let assets = cache.asset_objects();
        assert_eq!(assets.len(), 1);
        let logo = &assets[0];
        assert_eq!(logo.file_name, "logo");
        assert_eq!(logo.ext, "png");
        assert_eq!(logo.file_type, "Image");
        assert_eq!(logo.slug, "/img/logo.png");
        // DEFAULT_OUT_DIR is ".peisar" when outDir is not configured.
        assert_eq!(logo.out_file_path, ".peisar/img/logo.png");
        // binary raw is base64
        assert_eq!(logo.raw, binary_to_base64(&[1u8, 2, 3]));

        restore(orig, tmp);
        Ok(())
    }

    #[test]
    fn lru_eviction_falls_through_to_archive() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let (tmp, orig) = make_tmp("evict")?;
        env::set_current_dir(&tmp)?;
        write_md(&tmp, "docs/small.md", "# Small\n");
        write_md(
            &tmp,
            "docs/big.md",
            "# Big\nBody padding to exceed the budget.\n",
        );

        let cache = PeisarCache::with_config(PeisarCacheConfig {
            entry_dir: "docs".into(),
            memory: Some(MemoryConfig {
                byte_budget: Some(10),
                entry_budget: None,
            }),
            // Tiny thresholds so the flush happens quickly.
            archive: Some(ArchiveConfig {
                shard_bytes: Some(1),
                flush_interval_ms: Some(100),
            }),
            ..Default::default()
        })?;
        cache.flush();

        // The LRU evicted entries, but they are still readable (archive).
        let files = cache.markdown_files();
        assert_eq!(files.len(), 2, "both files still resolvable");
        for f in &files {
            assert!(cache.get_text(f.clone()).is_some(), "{} readable", f);
        }

        restore(orig, tmp);
        Ok(())
    }

    #[test]
    fn resolve_url_uses_hosting_config() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let (tmp, orig) = make_tmp("resolve")?;
        env::set_current_dir(&tmp)?;
        write_md(&tmp, "contents/index.md", "# Home\n");

        let cache = PeisarCache::with_config(PeisarCacheConfig {
            entry_dir: "contents".into(),
            hosting: Some(HostingConfig {
                provider: Some(HostingProvider::GithubPages),
                repo: Some("my-repo".into()),
                ..Default::default()
            }),
            ..Default::default()
        })?;
        assert_eq!(cache.base_url(), "/my-repo/");
        assert_eq!(
            cache.resolve_url("docs/intro".into()),
            "/my-repo/docs/intro"
        );

        restore(orig, tmp);
        Ok(())
    }

    #[test]
    fn gitignore_entry_not_duplicated() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let (tmp, orig) = make_tmp("gitignore")?;
        env::set_current_dir(&tmp)?;
        fs::write(".gitignore", format!("{ARCHIVE_DIR_NAME}\n"))?;
        write_md(&tmp, "docs/doc2.md", "# No frontmatter\nContent\n");

        let _cache = PeisarCache::new("docs")?;

        let git = fs::read_to_string(".gitignore")?;
        let occurrences = git.lines().filter(|l| l.trim() == ARCHIVE_DIR_NAME).count();
        assert_eq!(occurrences, 1);

        restore(orig, tmp);
        Ok(())
    }

    #[test]
    fn map_json_contains_frontmatter() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let (tmp, orig) = make_tmp("mapjson")?;
        env::set_current_dir(&tmp)?;
        write_md(
            &tmp,
            "docs/tagged.md",
            "---\ntitle: Tagged\ntags:\n  - x\n---\n# Tagged\n",
        );

        let cache = PeisarCache::new("docs")?;
        cache.flush();

        let map: serde_json::Value = serde_json::from_str(&cache.map_json()).unwrap();
        let entry = &map["files"]["docs/tagged.md"];
        assert_eq!(entry["frontmatter"]["title"], "Tagged");
        assert_eq!(entry["frontmatter"]["tags"][0], "x");
        assert_eq!(entry["kind"], "text");
        assert!(entry["hash"].as_str().unwrap().len() == 64);

        restore(orig, tmp);
        Ok(())
    }

    #[test]
    fn pretty_path_renders_relative_to_root() {
        let root = Path::new("/project");
        assert_eq!(
            pretty_path(Path::new("/project/contents/docs/a.md"), root),
            "contents/docs/a.md"
        );
        // Root-level paths render as their bare name.
        assert_eq!(
            pretty_path(Path::new("/project/README.md"), root),
            "README.md"
        );
        // Paths outside the root fall back to the absolute path.
        assert_eq!(
            pretty_path(Path::new("/elsewhere/b.png"), root),
            "/elsewhere/b.png"
        );
    }

    #[test]
    fn format_change_line_matches_kind_and_relative_path() {
        let root = Path::new("/project");
        let abs = root.join("contents").join("a.md");
        assert_eq!(
            format_change_line("create", &abs, root, false),
            "created contents/a.md"
        );
        assert_eq!(
            format_change_line("modify", &abs, root, false),
            "modified contents/a.md"
        );
        assert_eq!(
            format_change_line("remove", &abs, root, false),
            "removed contents/a.md"
        );
        assert_eq!(
            format_change_line("other", &abs, root, false),
            "changed contents/a.md"
        );
        // Colors wrap the label only; the path stays plain.
        let colored = format_change_line("modify", &abs, root, true);
        assert_eq!(
            colored,
            format!("{LOG_STYLE_YELLOW}modified{LOG_STYLE_RESET} contents/a.md")
        );
        // Paths outside the root fall back to the absolute path.
        let outside = Path::new("/elsewhere/b.png");
        assert_eq!(
            format_change_line("create", outside, root, false),
            "created /elsewhere/b.png"
        );
    }

    #[test]
    fn classify_change_uses_filesystem_state() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let (tmp, orig) = make_tmp("classify")?;
        env::set_current_dir(&tmp)?;
        write_md(&tmp, "docs/known.md", "# Known\n");

        let cache = PeisarCache::new("docs")?;

        // Exists + known to the cache → modify.
        assert_eq!(
            classify_change(
                &cache.memory,
                &cache.archive,
                &tmp.join("docs").join("known.md")
            ),
            "modify"
        );
        // Exists + unknown → create.
        write_md(&tmp, "docs/fresh.md", "# Fresh\n");
        assert_eq!(
            classify_change(
                &cache.memory,
                &cache.archive,
                &tmp.join("docs").join("fresh.md")
            ),
            "create"
        );
        // Gone → remove (even though never cached).
        assert_eq!(
            classify_change(
                &cache.memory,
                &cache.archive,
                &tmp.join("docs").join("ghost.md")
            ),
            "remove"
        );

        restore(orig, tmp);
        Ok(())
    }

    #[test]
    fn should_log_change_one_line_per_operation() {
        let now = Instant::now();
        let p = Path::new("/project/contents/a.md");
        let mut last: HashMap<PathBuf, Instant> = HashMap::new();

        // Untracked extensions (editor side files) never log.
        let tmp = Path::new("/project/contents/a.md.tmp");
        assert!(!should_log_change(tmp, &last, now));
        // A trackable path with no recent line logs; the duplicate
        // events of one operation (create/remove/modify within the
        // window) do not log again.
        assert!(should_log_change(p, &last, now));
        last.insert(p.to_path_buf(), now);
        assert!(!should_log_change(p, &last, now));
        // A second operation 2 s later is a new save — log it.
        let later = now + Duration::from_secs(2);
        assert!(should_log_change(p, &last, later));
    }

    #[test]
    fn log_trackable_matches_cache_admission() {
        // Markdown and asset extensions log; editor side files never.
        assert!(log_trackable(Path::new("a/hi.md")));
        assert!(log_trackable(Path::new("a/hi.markdown")));
        assert!(log_trackable(Path::new("a/logo.png")));
        assert!(log_trackable(Path::new("a/style.css")));
        assert!(!log_trackable(Path::new("a/hi.md.tmp")));
        assert!(!log_trackable(Path::new("a/hi.tmp")));
        assert!(!log_trackable(Path::new("a/.hi.md.swp")));
        assert!(!log_trackable(Path::new("a/hi")));
    }

    #[test]
    fn stop_watching_is_safe_without_starting() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let (tmp, orig) = make_tmp("stopwatch")?;
        env::set_current_dir(&tmp)?;
        write_md(&tmp, "docs/a.md", "# A\n");

        let mut cache = PeisarCache::new("docs")?;
        // Stop without start, then stop twice — all must be no-ops.
        cache.stop_watching();
        cache.start_watching()?;
        cache.stop_watching();
        cache.stop_watching();
        // The watcher is gone: the entry stayed in memory throughout.
        assert!(cache.get(&tmp.join("docs").join("a.md")).is_some());

        restore(orig, tmp);
        Ok(())
    }
}
