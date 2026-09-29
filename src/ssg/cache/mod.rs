use super::files::{
    ASSET_BINARY_EXTENSIONS, ASSET_EXTENSIONS, MARKDOWN_EXTENSIONS, collect_asset_files,
    collect_markdown_files,
};
use crate::frontmatter::parse_markdown_frontmatter;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock, mpsc};
use std::thread;

/// A file-change event delivered to JavaScript `onChange` callbacks.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct CacheChangeEvent {
    /// Absolute path of the changed file.
    pub path: String,
    /// `"create"`, `"modify"`, or `"remove"`.
    pub kind: String,
    /// `true` when the file is under the markdown entry directory.
    pub is_markdown: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct CacheEntry {
    file_path: String,
    markdown_raw_content: String,
    frontmatter_data: Option<Value>,
    #[serde(default)]
    is_binary: bool,
}
#[napi]
#[derive(Debug, Clone)]
pub enum CachedContent {
    Text(String),
    Binary(Vec<u8>),
}

/// JavaScript callback invoked on every cache change while watching.
pub type JsChangeCallback = Box<dyn Fn(CacheChangeEvent) + Send + 'static>;

/// Shared registry of JS change callbacks, keyed by subscription id.
#[derive(Clone, Default)]
struct ChangeCallbacks {
    inner: Arc<std::sync::Mutex<HashMap<u32, JsChangeCallback>>>,
    next_id: Arc<AtomicU32>,
}

impl ChangeCallbacks {
    fn add(&self, cb: JsChangeCallback) -> u32 {
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

#[derive(Debug)]
enum PersistCommand {
    Update(PathBuf, CachedContent),
    Remove(PathBuf),
    SyncAll(HashMap<PathBuf, CachedContent>),
}

fn compute_target_for_source(src: &Path, cwd: &Path, is_binary: bool) -> (PathBuf, PathBuf) {
    // returns (target_path, rel_path)
    let rel_path = match src.strip_prefix(cwd) {
        Ok(rel) => rel.to_path_buf(),
        Err(_) => src.to_path_buf(),
    };
    let parent = rel_path.parent().map(|p| p.to_path_buf());
    let fname = rel_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    let cache_dir = cwd.join(".peisar_cache");

    let target = if is_binary {
        // mirror original filename for binary assets
        match &parent {
            Some(p) => cache_dir.join(p).join(&fname),
            None => cache_dir.join(&fname),
        }
    } else {
        // text entries are stored in the configured cache format (default: JSON).
        // Set PEISAR_CACHE_FORMAT=bincode to store compact binary entries (.bin).
        let fmt = env::var("PEISAR_CACHE_FORMAT").unwrap_or_else(|_| "json".to_string());
        let ext = if fmt.eq_ignore_ascii_case("bincode") {
            ".bin"
        } else {
            ".json"
        };
        let cache_name = format!("{}{}", fname, ext);
        match &parent {
            Some(p) => cache_dir.join(p).join(&cache_name),
            None => cache_dir.join(&cache_name),
        }
    };

    (target, rel_path)
}

fn persist_cache_map(map: &HashMap<PathBuf, CachedContent>) -> io::Result<()> {
    let cwd = env::current_dir()?;
    let cache_dir = cwd.join(".peisar_cache");
    fs::create_dir_all(&cache_dir)?;

    let mut expected_files: HashSet<PathBuf> = HashSet::new();

    for (path, content) in map {
        match content {
            CachedContent::Text(text) => {
                let (target, rel_path) = compute_target_for_source(path, &cwd, false);
                let target_dir = target
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or(cache_dir.clone());
                fs::create_dir_all(&target_dir)?;

                let file_path_str = rel_path.to_string_lossy().to_string();
                let fm = parse_markdown_frontmatter(&text).unwrap();
                let entry = CacheEntry {
                    file_path: file_path_str,
                    markdown_raw_content: text.clone(),
                    frontmatter_data: fm.yaml_data,
                    is_binary: false,
                };

                // Support configurable on-disk format via PEISAR_CACHE_FORMAT.
                let fmt = env::var("PEISAR_CACHE_FORMAT").unwrap_or_else(|_| "json".to_string());
                if fmt.eq_ignore_ascii_case("bincode") {
                    let bytes = bincode::serialize(&entry).map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("bincode error: {}", e))
                    })?;

                    // Write atomically to a temp file then rename
                    let fname = target
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let tmp_name = format!("{}.tmp", fname);
                    let tmp_path = target_dir.join(&tmp_name);
                    fs::write(&tmp_path, &bytes)?;
                    fs::rename(&tmp_path, &target)?;
                } else {
                    let json = serde_json::to_string_pretty(&entry).map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("serde_json error: {}", e))
                    })?;

                    // Write atomically to a temp file then rename
                    let fname = target
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let tmp_name = format!("{}.tmp", fname);
                    let tmp_path = target_dir.join(&tmp_name);
                    fs::write(&tmp_path, json.as_bytes())?;
                    fs::rename(&tmp_path, &target)?;
                }

                expected_files.insert(target);
            }
            CachedContent::Binary(bytes) => {
                let (target, _rel_path) = compute_target_for_source(path, &cwd, true);
                let target_dir = target
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or(cache_dir.clone());
                fs::create_dir_all(&target_dir)?;

                let fname = target
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                let tmp_name = format!("{}.tmp", fname);
                let tmp_path = target_dir.join(&tmp_name);
                fs::write(&tmp_path, bytes)?;
                fs::rename(&tmp_path, &target)?;

                expected_files.insert(target);
            }
        }
    }

    // Remove stale files that are present in .peisar_cache but not expected
    fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {
                collect_files(&p, out)?;
            } else if p.is_file() {
                out.push(p);
            }
        }
        Ok(())
    }

    if cache_dir.exists() {
        let mut existing_files: Vec<PathBuf> = Vec::new();
        collect_files(&cache_dir, &mut existing_files)?;
        for f in existing_files {
            if !expected_files.contains(&f) {
                let _ = fs::remove_file(&f);
            }
        }

        // Remove empty directories under .peisar_cache
        fn remove_empty(dir: &Path) -> io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let p = entry.path();
                if p.is_dir() {
                    remove_empty(&p)?;
                    if fs::read_dir(&p)?.next().is_none() {
                        let _ = fs::remove_dir(&p);
                    }
                }
            }
            Ok(())
        }
        let _ = remove_empty(&cache_dir);
    }

    // Ensure .peisar_cache is in .gitignore
    let gitignore_path = cwd.join(".gitignore");
    if gitignore_path.exists() {
        let existing = fs::read_to_string(&gitignore_path)?;
        let mut has_entry = false;
        for line in existing.lines() {
            if line.trim() == ".peisar_cache" {
                has_entry = true;
                break;
            }
        }
        if !has_entry {
            let mut file = OpenOptions::new().append(true).open(&gitignore_path)?;
            if !existing.ends_with('\n') {
                file.write_all(b"\n")?;
            }
            file.write_all(b".peisar_cache\n")?;
        }
    } else {
        fs::write(gitignore_path, ".peisar_cache\n")?;
    }

    Ok(())
}

/// In-memory cache of markdown files under a given directory.
/// The cache maps absolute PathBuf -> raw file contents.
#[napi]
pub struct PeisarCache {
    // cache maps source path -> cached content (text or binary)
    cache: Arc<RwLock<HashMap<PathBuf, CachedContent>>>,
    watcher: Option<RecommendedWatcher>,
    entry_dir: PathBuf,
    assets_dir: Option<PathBuf>,
    // Sender for persistence commands — None until worker is started in new()
    persist_tx: Option<mpsc::Sender<PersistCommand>>,
    // Background worker handle to join on drop
    worker_handle: Option<thread::JoinHandle<()>>,
    // JS onChange subscriptions fed by the watcher
    change_callbacks: ChangeCallbacks,
}
#[napi]
impl PeisarCache {
    /// Construct a new PeisarCache using the default discovery behavior.
    ///
    /// This is equivalent to calling `with_config(entry_dir, None)` and will
    /// use the "public" directory at project root when present.
    ///
    /// JS: `new PeisarCache(entryDir, assetsDir?)` — both plain strings.
    #[napi(constructor)]
    pub fn new_js(entry_dir: String, assets_dir: Option<String>) -> Result<Self> {
        Self::with_config_js(entry_dir, assets_dir)
            .map_err(|e| napi::Error::new(napi::Status::GenericFailure, format!("{}", e)))
    }

    /// Rust-generic constructor kept for the crate's internal users and tests.
    pub fn new<P: AsRef<Path>>(entry_dir: P) -> io::Result<Self> {
        PeisarCache::with_config(entry_dir, None::<&Path>)
    }

    /// JS: same as the constructor, for callers that prefer a factory shape.
    /// Kept non-generic so NAPI can export it.
    #[napi(factory)]
    pub fn with_config_js(entry_dir: String, assets_dir: Option<String>) -> Result<Self> {
        PeisarCache::with_config(entry_dir, assets_dir)
            .map_err(|e| napi::Error::new(napi::Status::GenericFailure, format!("{}", e)))
    }

    /// Load all markdown files under `entry_dir` into the cache.
    /// Also persists the cache to the `.peisar_cache` directory and ensures
    /// the `.gitignore` contains an entry for `.peisar_cache`.
    /// Construct a PeisarCache with an optional custom assets directory.
    ///
    /// If `assets_dir` is `Some(path)`, that path (absolute or relative to
    /// current working directory) will be used to discover assets. If
    /// `assets_dir` is `None`, the default behavior is to use the "public"
    /// directory at project root when present.
    pub fn with_config<P: AsRef<Path>, Q: AsRef<Path>>(
        entry_dir: P,
        assets_dir: Option<Q>,
    ) -> io::Result<Self> {
        let entry_dir = entry_dir.as_ref().to_path_buf();

        // Build initial in-memory cache with markdown files (text) and
        // optionally assets under the provided assets_dir (binary/text
        // depending on extension).
        let mut cache_map: HashMap<PathBuf, CachedContent> = HashMap::new();

        let files = collect_markdown_files(&entry_dir)?;
        for f in files {
            if let Ok(text) = fs::read_to_string(&f) {
                cache_map.insert(f, CachedContent::Text(text));
            }
        }

        // Resolve assets_dir: use provided path when given; otherwise fall
        // back to "public" at project root if present.
        let cwd = env::current_dir()?;
        let resolved_assets: Option<PathBuf> = match assets_dir {
            Some(p) => {
                let pbuf = p.as_ref().to_path_buf();
                if pbuf.is_absolute() {
                    Some(pbuf)
                } else {
                    Some(cwd.join(pbuf))
                }
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

        if let Some(ref a_dir) = resolved_assets {
            if let Ok(asset_files) = collect_asset_files(a_dir) {
                for f in asset_files {
                    // determine if binary by extension
                    let is_binary = f
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| ASSET_BINARY_EXTENSIONS.contains(&s.to_lowercase().as_str()))
                        .unwrap_or(false);

                    if is_binary {
                        if let Ok(bytes) = fs::read(&f) {
                            cache_map.insert(f, CachedContent::Binary(bytes));
                        }
                    } else {
                        if let Ok(text) = fs::read_to_string(&f) {
                            cache_map.insert(f, CachedContent::Text(text));
                        }
                    }
                }
            }
        }

        // Persist the initial cache to disk synchronously so callers observe
        // cache files immediately after construction.
        persist_cache_map(&cache_map)?;

        // Prepare background persistence worker that accepts incremental
        // commands so the watcher callback can remain non-blocking for large
        // repositories. Seed the worker's hash map with the current contents
        // to enable content-hash-based de-duplication.
        let (tx, rx) = mpsc::channel::<PersistCommand>();

        let mut last_hashes: HashMap<PathBuf, String> = HashMap::new();
        for (src, content) in &cache_map {
            let (target, _rel) =
                compute_target_for_source(src, &cwd, matches!(content, CachedContent::Binary(_)));
            let hash = match content {
                CachedContent::Text(s) => blake3::hash(s.as_bytes()).to_hex().to_string(),
                CachedContent::Binary(b) => blake3::hash(b).to_hex().to_string(),
            };
            last_hashes.insert(target, hash);
        }

        let handle = thread::spawn(move || {
            let cache_dir = cwd.join(".peisar_cache");

            // helper to collect all files under cache_dir
            fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let p = entry.path();
                    if p.is_dir() {
                        collect_files(&p, out)?;
                    } else if p.is_file() {
                        out.push(p);
                    }
                }
                Ok(())
            }

            let mut hashes = last_hashes;

            while let Ok(cmd) = rx.recv() {
                match cmd {
                    PersistCommand::Update(src, content) => {
                        let is_binary = matches!(content, CachedContent::Binary(_));
                        let (target, rel_path) = compute_target_for_source(&src, &cwd, is_binary);
                        let target_dir = target
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or(cache_dir.clone());
                        if let Err(e) = fs::create_dir_all(&target_dir) {
                            eprintln!("persist mkdir error: {:?}", e);
                            continue;
                        }

                        let hash = match &content {
                            CachedContent::Text(s) => {
                                blake3::hash(s.as_bytes()).to_hex().to_string()
                            }
                            CachedContent::Binary(b) => blake3::hash(b).to_hex().to_string(),
                        };
                        if let Some(prev) = hashes.get(&target) {
                            if prev == &hash {
                                // nothing to do
                                continue;
                            }
                        }

                        if is_binary {
                            if let CachedContent::Binary(bytes) = content {
                                let fname = target
                                    .file_name()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                let tmp_name = format!("{}.tmp", fname);
                                let tmp_path = target_dir.join(&tmp_name);
                                if let Err(e) = fs::write(&tmp_path, &bytes) {
                                    eprintln!("persist write tmp error: {:?}", e);
                                    continue;
                                }
                                if let Err(e) = fs::rename(&tmp_path, &target) {
                                    eprintln!("persist rename error: {:?}", e);
                                    let _ = fs::remove_file(&tmp_path);
                                    continue;
                                }
                                hashes.insert(target, hash);
                            }
                        } else {
                            if let CachedContent::Text(text) = content {
                                let fm = parse_markdown_frontmatter(&text).unwrap();
                                let entry = CacheEntry {
                                    file_path: rel_path.to_string_lossy().to_string(),
                                    markdown_raw_content: text.clone(),
                                    frontmatter_data: fm.yaml_data,
                                    is_binary: false,
                                };

                                let fmt = env::var("PEISAR_CACHE_FORMAT")
                                    .unwrap_or_else(|_| "json".to_string());
                                if fmt.eq_ignore_ascii_case("bincode") {
                                    match bincode::serialize(&entry) {
                                        Ok(bytes) => {
                                            let fname = target
                                                .file_name()
                                                .map(|f| f.to_string_lossy().to_string())
                                                .unwrap_or_default();
                                            let tmp_name = format!("{}.tmp", fname);
                                            let tmp_path = target_dir.join(&tmp_name);
                                            if let Err(e) = fs::write(&tmp_path, &bytes) {
                                                eprintln!("persist write tmp error: {:?}", e);
                                                continue;
                                            }
                                            if let Err(e) = fs::rename(&tmp_path, &target) {
                                                eprintln!("persist rename error: {:?}", e);
                                                let _ = fs::remove_file(&tmp_path);
                                                continue;
                                            }
                                            hashes.insert(target, hash);
                                        }
                                        Err(e) => eprintln!("bincode serialize error: {:?}", e),
                                    }
                                } else {
                                    match serde_json::to_string_pretty(&entry) {
                                        Ok(json) => {
                                            let fname = target
                                                .file_name()
                                                .map(|f| f.to_string_lossy().to_string())
                                                .unwrap_or_default();
                                            let tmp_name = format!("{}.tmp", fname);
                                            let tmp_path = target_dir.join(&tmp_name);
                                            if let Err(e) = fs::write(&tmp_path, json.as_bytes()) {
                                                eprintln!("persist write tmp error: {:?}", e);
                                                continue;
                                            }
                                            if let Err(e) = fs::rename(&tmp_path, &target) {
                                                eprintln!("persist rename error: {:?}", e);
                                                let _ = fs::remove_file(&tmp_path);
                                                continue;
                                            }
                                            hashes.insert(target, hash);
                                        }
                                        Err(e) => eprintln!("serde_json error: {:?}", e),
                                    }
                                }
                            }
                        }
                    }
                    PersistCommand::Remove(src) => {
                        // Try removing both possible targets (text .json and mirrored binary file)
                        let (target_text, _rel) = compute_target_for_source(&src, &cwd, false);
                        if target_text.exists() {
                            if let Err(e) = fs::remove_file(&target_text) {
                                eprintln!("persist remove error: {:?}", e);
                            }
                        }
                        hashes.remove(&target_text);

                        let (target_bin, _rel2) = compute_target_for_source(&src, &cwd, true);
                        if target_bin.exists() {
                            if let Err(e) = fs::remove_file(&target_bin) {
                                eprintln!("persist remove error: {:?}", e);
                            }
                        }
                        hashes.remove(&target_bin);

                        // clean up empty parent directories for both targets
                        for target in &[target_text, target_bin] {
                            if let Some(mut parent) = target.parent().map(|p| p.to_path_buf()) {
                                while parent.starts_with(&cache_dir)
                                    && fs::read_dir(&parent)
                                        .map(|mut it| it.next().is_none())
                                        .unwrap_or(false)
                                {
                                    let _ = fs::remove_dir(&parent);
                                    if let Some(p) = parent.parent() {
                                        parent = p.to_path_buf();
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    PersistCommand::SyncAll(map) => {
                        let mut expected: HashSet<PathBuf> = HashSet::new();
                        for (src, content) in map {
                            let is_binary = matches!(content, CachedContent::Binary(_));
                            let (target, rel_path) =
                                compute_target_for_source(&src, &cwd, is_binary);
                            expected.insert(target.clone());

                            let target_dir = target
                                .parent()
                                .map(|p| p.to_path_buf())
                                .unwrap_or(cache_dir.clone());
                            if let Err(e) = fs::create_dir_all(&target_dir) {
                                eprintln!("persist mkdir error: {:?}", e);
                                continue;
                            }

                            let hash = match &content {
                                CachedContent::Text(s) => {
                                    blake3::hash(s.as_bytes()).to_hex().to_string()
                                }
                                CachedContent::Binary(b) => blake3::hash(b).to_hex().to_string(),
                            };
                            if let Some(prev) = hashes.get(&target) {
                                if prev == &hash {
                                    continue;
                                }
                            }

                            if is_binary {
                                if let CachedContent::Binary(bytes) = content {
                                    let fname = target
                                        .file_name()
                                        .map(|f| f.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    let tmp_name = format!("{}.tmp", fname);
                                    let tmp_path = target_dir.join(&tmp_name);
                                    if let Err(e) = fs::write(&tmp_path, &bytes) {
                                        eprintln!("persist write tmp error: {:?}", e);
                                        continue;
                                    }
                                    if let Err(e) = fs::rename(&tmp_path, &target) {
                                        eprintln!("persist rename error: {:?}", e);
                                        let _ = fs::remove_file(&tmp_path);
                                        continue;
                                    }
                                    hashes.insert(target, hash);
                                }
                            } else {
                                if let CachedContent::Text(text) = content {
                                    let fm = parse_markdown_frontmatter(&text).unwrap();
                                    let entry = CacheEntry {
                                        file_path: rel_path.to_string_lossy().to_string(),
                                        markdown_raw_content: text.clone(),
                                        frontmatter_data: fm.yaml_data,
                                        is_binary: false,
                                    };

                                    let fmt = env::var("PEISAR_CACHE_FORMAT")
                                        .unwrap_or_else(|_| "json".to_string());
                                    if fmt.eq_ignore_ascii_case("bincode") {
                                        match bincode::serialize(&entry) {
                                            Ok(bytes) => {
                                                let fname = target
                                                    .file_name()
                                                    .map(|f| f.to_string_lossy().to_string())
                                                    .unwrap_or_default();
                                                let tmp_name = format!("{}.tmp", fname);
                                                let tmp_path = target_dir.join(&tmp_name);
                                                if let Err(e) = fs::write(&tmp_path, &bytes) {
                                                    eprintln!("persist write tmp error: {:?}", e);
                                                    continue;
                                                }
                                                if let Err(e) = fs::rename(&tmp_path, &target) {
                                                    eprintln!("persist rename error: {:?}", e);
                                                    let _ = fs::remove_file(&tmp_path);
                                                    continue;
                                                }
                                                hashes.insert(target, hash);
                                            }
                                            Err(e) => eprintln!("bincode serialize error: {:?}", e),
                                        }
                                    } else {
                                        match serde_json::to_string_pretty(&entry) {
                                            Ok(json) => {
                                                let fname = target
                                                    .file_name()
                                                    .map(|f| f.to_string_lossy().to_string())
                                                    .unwrap_or_default();
                                                let tmp_name = format!("{}.tmp", fname);
                                                let tmp_path = target_dir.join(&tmp_name);
                                                if let Err(e) =
                                                    fs::write(&tmp_path, json.as_bytes())
                                                {
                                                    eprintln!("persist write tmp error: {:?}", e);
                                                    continue;
                                                }
                                                if let Err(e) = fs::rename(&tmp_path, &target) {
                                                    eprintln!("persist rename error: {:?}", e);
                                                    let _ = fs::remove_file(&tmp_path);
                                                    continue;
                                                }
                                                hashes.insert(target, hash);
                                            }
                                            Err(e) => eprintln!("serde_json error: {:?}", e),
                                        }
                                    }
                                }
                            }
                        }

                        // remove stale
                        if cache_dir.exists() {
                            let mut existing: Vec<PathBuf> = Vec::new();
                            if let Err(e) = collect_files(&cache_dir, &mut existing) {
                                eprintln!("collect files error: {:?}", e);
                            } else {
                                for f in existing {
                                    if !expected.contains(&f) {
                                        let _ = fs::remove_file(&f);
                                        hashes.remove(&f);
                                    }
                                }
                            }

                            // remove empty dirs
                            fn remove_empty(dir: &Path) -> io::Result<()> {
                                for entry in fs::read_dir(dir)? {
                                    let entry = entry?;
                                    let p = entry.path();
                                    if p.is_dir() {
                                        remove_empty(&p)?;
                                        if fs::read_dir(&p)?.next().is_none() {
                                            let _ = fs::remove_dir(&p);
                                        }
                                    }
                                }
                                Ok(())
                            }
                            let _ = remove_empty(&cache_dir);
                        }
                    }
                }
            }
        });

        Ok(PeisarCache {
            cache: Arc::new(RwLock::new(cache_map)),
            watcher: None,
            entry_dir,
            assets_dir: resolved_assets,
            persist_tx: Some(tx),
            worker_handle: Some(handle),
            change_callbacks: ChangeCallbacks::default(),
        })
    }

    /// Start watching the entry_dir recursively. The watcher will update the
    /// in-memory cache on create/modify/remove events for markdown files and
    /// assets, and persist updates to disk.
    ///
    /// The returned Result is only for watcher setup errors; runtime errors are
    /// printed to stderr by the watch callback.
    ///
    /// JS: `cache.startWatching()` — errors surface as JS exceptions.
    #[napi]
    pub fn start_watching_js(&mut self) -> Result<()> {
        self.start_watching()
            .map_err(|e| napi::Error::new(napi::Status::GenericFailure, format!("{}", e)))
    }

    pub fn start_watching(&mut self) -> io::Result<()> {
        let cache = Arc::clone(&self.cache);
        let entry_dir = self.entry_dir.clone();
        let assets_dir = self.assets_dir.clone();
        // Shared with the watcher thread so JS onChange callbacks fire on
        // every detected create/modify/remove event.
        let change_callbacks = self.change_callbacks.clone();

        let tx = match &self.persist_tx {
            Some(t) => t.clone(),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "persistence worker not started",
                ));
            }
        };

        // recommended_watcher takes a closure that's invoked on file events.
        let mut watcher: RecommendedWatcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                match res {
                    Ok(event) => {
                        let kind = event.kind.clone();
                        for path in event.paths {
                            // Notify JS subscribers about this change first,
                            // then update the in-memory cache below.
                            let is_markdown = path.starts_with(&entry_dir);
                            let kind_str = if matches!(kind, notify::EventKind::Create(_)) {
                                "create"
                            } else if matches!(kind, notify::EventKind::Modify(_)) {
                                "modify"
                            } else if matches!(kind, notify::EventKind::Remove(_)) {
                                "remove"
                            } else {
                                "other"
                            };
                            change_callbacks.dispatch(CacheChangeEvent {
                                path: path.to_string_lossy().to_string(),
                                kind: kind_str.to_string(),
                                is_markdown,
                            });
                            // If the event targets a directory, rescan everything.
                            if path.is_dir() {
                                let mut new_map: HashMap<PathBuf, CachedContent> = HashMap::new();

                                if let Ok(files) = collect_markdown_files(&entry_dir) {
                                    for f in files {
                                        if let Ok(text) = fs::read_to_string(&f) {
                                            new_map.insert(f, CachedContent::Text(text));
                                        }
                                    }
                                }

                                if let Some(ref a_dir) = assets_dir {
                                    if let Ok(asset_files) = collect_asset_files(a_dir) {
                                        for f in asset_files {
                                            let ext_l = f
                                                .extension()
                                                .and_then(|e| e.to_str())
                                                .map(|s| s.to_lowercase())
                                                .unwrap_or_default();
                                            if ASSET_EXTENSIONS.contains(&ext_l.as_str()) {
                                                let is_binary = ASSET_BINARY_EXTENSIONS
                                                    .contains(&ext_l.as_str());
                                                if is_binary {
                                                    if let Ok(bytes) = fs::read(&f) {
                                                        new_map.insert(
                                                            f,
                                                            CachedContent::Binary(bytes),
                                                        );
                                                    }
                                                } else if let Ok(text) = fs::read_to_string(&f) {
                                                    new_map.insert(f, CachedContent::Text(text));
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Ok(mut w) = cache.write() {
                                    *w = new_map.clone();
                                    if let Err(e) = tx.send(PersistCommand::SyncAll(new_map)) {
                                        eprintln!("persist send error: {:?}", e);
                                    }
                                }

                                continue;
                            }

                            if path.is_file() {
                                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                    let ext_l = ext.to_lowercase();

                                    // Markdown file
                                    if MARKDOWN_EXTENSIONS.contains(&ext_l.as_str()) {
                                        match fs::read_to_string(&path) {
                                            Ok(text) => {
                                                if let Ok(mut w) = cache.write() {
                                                    w.insert(
                                                        path.clone(),
                                                        CachedContent::Text(text.clone()),
                                                    );
                                                    if let Err(e) = tx.send(PersistCommand::Update(
                                                        path.clone(),
                                                        CachedContent::Text(text),
                                                    )) {
                                                        eprintln!("persist send error: {:?}", e);
                                                    }
                                                }
                                            }
                                            Err(_) => {
                                                if let Ok(mut w) = cache.write() {
                                                    w.remove(&path);
                                                    if let Err(e) = tx
                                                        .send(PersistCommand::Remove(path.clone()))
                                                    {
                                                        eprintln!("persist send error: {:?}", e);
                                                    }
                                                }
                                            }
                                        }
                                    // Asset file
                                    } else if ASSET_EXTENSIONS.contains(&ext_l.as_str()) {
                                        let is_binary =
                                            ASSET_BINARY_EXTENSIONS.contains(&ext_l.as_str());
                                        if is_binary {
                                            match fs::read(&path) {
                                                Ok(bytes) => {
                                                    if let Ok(mut w) = cache.write() {
                                                        w.insert(
                                                            path.clone(),
                                                            CachedContent::Binary(bytes.clone()),
                                                        );
                                                        if let Err(e) =
                                                            tx.send(PersistCommand::Update(
                                                                path.clone(),
                                                                CachedContent::Binary(bytes),
                                                            ))
                                                        {
                                                            eprintln!(
                                                                "persist send error: {:?}",
                                                                e
                                                            );
                                                        }
                                                    }
                                                }
                                                Err(_) => {
                                                    if let Ok(mut w) = cache.write() {
                                                        w.remove(&path);
                                                        if let Err(e) = tx.send(
                                                            PersistCommand::Remove(path.clone()),
                                                        ) {
                                                            eprintln!(
                                                                "persist send error: {:?}",
                                                                e
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            match fs::read_to_string(&path) {
                                                Ok(text) => {
                                                    if let Ok(mut w) = cache.write() {
                                                        w.insert(
                                                            path.clone(),
                                                            CachedContent::Text(text.clone()),
                                                        );
                                                        if let Err(e) =
                                                            tx.send(PersistCommand::Update(
                                                                path.clone(),
                                                                CachedContent::Text(text),
                                                            ))
                                                        {
                                                            eprintln!(
                                                                "persist send error: {:?}",
                                                                e
                                                            );
                                                        }
                                                    }
                                                }
                                                Err(_) => {
                                                    if let Ok(mut w) = cache.write() {
                                                        w.remove(&path);
                                                        if let Err(e) = tx.send(
                                                            PersistCommand::Remove(path.clone()),
                                                        ) {
                                                            eprintln!(
                                                                "persist send error: {:?}",
                                                                e
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("watch error: {:?}", e),
                }
            })
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("notify::recommended_watcher error: {}", e),
                )
            })?;

        watcher
            .watch(&self.entry_dir, RecursiveMode::Recursive)
            .map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("notify::watch error: {}", e))
            })?;

        if let Some(a) = &self.assets_dir {
            watcher.watch(a, RecursiveMode::Recursive).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("notify::watch error: {}", e))
            })?;
        }

        self.watcher = Some(watcher);
        Ok(())
    }

    /// Get a cached content entry by path, if present.
    ///
    /// Returns Some(CachedContent) for both textual and binary entries. This is
    /// a backwards-incompatible change from the previous API that returned an
    /// Option<String> for textual entries only.
    pub fn get(&self, path: &Path) -> Option<CachedContent> {
        match self.cache.read() {
            Ok(r) => r.get(path).cloned(),
            Err(_) => None,
        }
    }

    /// Return a clone of the entire cache map for textual entries only.
    pub fn all(&self) -> HashMap<PathBuf, String> {
        match self.cache.read() {
            Ok(r) => {
                let mut out = HashMap::new();
                for (k, v) in r.iter() {
                    if let CachedContent::Text(s) = v {
                        out.insert(k.clone(), s.clone());
                    }
                }
                out
            }
            Err(_) => HashMap::new(),
        }
    }

    /// ----------------------------------------------------------------
    /// JavaScript (NAPI) surface
    /// ----------------------------------------------------------------
    /// All JS methods take/return plain strings because `&Path`/`PathBuf`
    /// do not cross the NAPI boundary.

    /// JS: `cache.getText(absPath)` — cached text of a file, or null.
    #[napi]
    pub fn get_text(&self, abs_path: String) -> Option<String> {
        match self.cache.read() {
            Ok(r) => match r.get(Path::new(&abs_path)) {
                Some(CachedContent::Text(s)) => Some(s.clone()),
                _ => None,
            },
            Err(_) => None,
        }
    }

    /// JS: `cache.getBinary(absPath)` — cached bytes of a binary asset, or null.
    #[napi]
    pub fn get_binary(&self, abs_path: String) -> Option<Vec<u8>> {
        match self.cache.read() {
            Ok(r) => match r.get(Path::new(&abs_path)) {
                Some(CachedContent::Binary(b)) => Some(b.clone()),
                _ => None,
            },
            Err(_) => None,
        }
    }

    /// JS: `cache.listFiles()` — absolute paths of everything cached.
    #[napi]
    pub fn list_files(&self) -> Vec<String> {
        match self.cache.read() {
            Ok(r) => r.keys().map(|p| p.to_string_lossy().to_string()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// JS: `cache.markdownFiles()` — absolute paths of cached markdown files.
    #[napi]
    pub fn markdown_files(&self) -> Vec<String> {
        self.filter_cached_files(|ext| MARKDOWN_EXTENSIONS.contains(&ext))
    }

    /// JS: `cache.assetFiles()` — absolute paths of cached non-markdown files.
    #[napi]
    pub fn asset_files(&self) -> Vec<String> {
        self.filter_cached_files(|ext| !MARKDOWN_EXTENSIONS.contains(&ext))
    }

    /// Filter cached keys by extension predicate (helper shared by
    /// `markdown_files` / `asset_files`).
    fn filter_cached_files(&self, keep: fn(&str) -> bool) -> Vec<String> {
        match self.cache.read() {
            Ok(r) => r
                .keys()
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| keep(e))
                        .unwrap_or(false)
                })
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// JS: `cache.onChange(cb)` — invoke `cb(event)` on every file change the
    /// watcher detects while watching is active. Returns a subscription id
    /// that can be passed to `offChange(id)`.
    ///
    /// The callback is wrapped in a `ThreadsafeFunction` because notify
    /// events fire on the watcher thread, not the JS main thread.
    #[napi]
    pub fn on_change(
        &self,
        #[napi(ts_arg_type = "(event: CacheChangeEvent) => void")] callback: Function<
            'static,
            CacheChangeEvent,
            Unknown<'static>,
        >,
    ) -> Result<u32> {
        // `ThreadsafeFunction` is required: notify events fire on the watcher
        // thread, not the JS main thread. `napi::threadsafe_function` is not
        // re-exported through `bindgen_prelude`, so use the full path.
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

    /// JS: `cache.offChange(id)` — remove a previously registered callback.
    #[napi]
    pub fn off_change(&self, id: u32) {
        self.change_callbacks.remove(id);
    }

    /// JS: `cache.dispose()` — stop the watcher and drop JS change callbacks.
    /// Safe to call more than once.
    #[napi]
    pub fn dispose(&mut self) {
        self.watcher = None;
        self.change_callbacks.clear();
    }
}

impl Drop for PeisarCache {
    fn drop(&mut self) {
        // Closing the sender will cause the worker thread to exit.
        self.persist_tx.take();
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::sync::Mutex;

    static TEST_CWD_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_persist_cache_and_gitignore_created() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let orig = env::current_dir()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("time error: {}", e)))?
            .as_millis();
        let tmp = env::temp_dir().join(format!("peisar_test_{}", now));
        if tmp.exists() {
            fs::remove_dir_all(&tmp)?;
        }
        fs::create_dir_all(&tmp)?;
        env::set_current_dir(&tmp)?;

        fs::create_dir_all("docs")?;
        let md_path = tmp.join("docs").join("doc1.md");
        let mut f = File::create(&md_path)?;
        f.write_all(b"---\ntitle: Test\ntags:\n  - a\n  - b\n---\n# Hello\n")?;
        f.sync_all()?;

        let _cache = PeisarCache::new("docs")?;

        let cache_file = tmp.join(".peisar_cache").join("docs").join("doc1.md.json");
        assert!(cache_file.exists());
        let s = fs::read_to_string(&cache_file)?;
        let entry: CacheEntry = serde_json::from_str(&s).unwrap();
        assert!(
            entry.file_path.ends_with("docs/doc1.md") || entry.file_path.ends_with("docs\\doc1.md")
        );
        assert!(entry.frontmatter_data.is_some());

        let gitignore = tmp.join(".gitignore");
        assert!(gitignore.exists());
        let git = fs::read_to_string(&gitignore)?;
        assert!(git.lines().any(|l| l.trim() == ".peisar_cache"));

        env::set_current_dir(orig)?;
        fs::remove_dir_all(&tmp)?;
        Ok(())
    }

    #[test]
    fn test_gitignore_not_duplicated() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let orig = env::current_dir()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("time error: {}", e)))?
            .as_millis();
        let tmp = env::temp_dir().join(format!("peisar_test_{}", now));
        if tmp.exists() {
            fs::remove_dir_all(&tmp)?;
        }
        fs::create_dir_all(&tmp)?;
        env::set_current_dir(&tmp)?;

        // pre-create .gitignore with entry
        fs::write(".gitignore", ".peisar_cache\n")?;

        fs::create_dir_all("docs")?;
        let md_path = tmp.join("docs").join("doc2.md");
        let mut f = File::create(&md_path)?;
        f.write_all(b"# No frontmatter\nContent\n")?;
        f.sync_all()?;

        let _cache = PeisarCache::new("docs")?;

        let git = fs::read_to_string(".gitignore")?;
        let occurrences = git.lines().filter(|l| l.trim() == ".peisar_cache").count();
        assert_eq!(occurrences, 1);

        env::set_current_dir(orig)?;
        fs::remove_dir_all(&tmp)?;
        Ok(())
    }

    #[test]
    fn test_stale_cache_removal_on_remove_command() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let orig = env::current_dir()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("time error: {}", e)))?
            .as_millis();
        let tmp = env::temp_dir().join(format!("peisar_test_{}", now));
        if tmp.exists() {
            fs::remove_dir_all(&tmp)?;
        }
        fs::create_dir_all(&tmp)?;
        env::set_current_dir(&tmp)?;

        fs::create_dir_all("docs")?;
        let md1 = tmp.join("docs").join("a.md");
        let md2 = tmp.join("docs").join("b.md");
        {
            let mut f1 = File::create(&md1)?;
            f1.write_all(b"# A\n")?;
            f1.sync_all()?;
            let mut f2 = File::create(&md2)?;
            f2.write_all(b"# B\n")?;
            f2.sync_all()?;
        }

        let cache = PeisarCache::new("docs")?;

        let cache_a = tmp.join(".peisar_cache").join("docs").join("a.md.json");
        let cache_b = tmp.join(".peisar_cache").join("docs").join("b.md.json");
        assert!(cache_a.exists());
        assert!(cache_b.exists());

        // Remove source file and send Remove command to worker
        fs::remove_file(&md1)?;
        if let Some(tx) = &cache.persist_tx {
            tx.send(PersistCommand::Remove(md1.clone())).unwrap();
        }

        // Wait for worker to process
        let mut waited = 0u32;
        while cache_a.exists() && waited < 50 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            waited += 1;
        }

        assert!(!cache_a.exists());

        env::set_current_dir(orig)?;
        fs::remove_dir_all(&tmp)?;
        Ok(())
    }

    #[test]
    fn test_hashing_avoids_rewrite() -> io::Result<()> {
        let _guard = TEST_CWD_LOCK.lock().unwrap();
        let orig = env::current_dir()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("time error: {}", e)))?
            .as_millis();
        let tmp = env::temp_dir().join(format!("peisar_test_{}", now));
        if tmp.exists() {
            fs::remove_dir_all(&tmp)?;
        }
        fs::create_dir_all(&tmp)?;
        env::set_current_dir(&tmp)?;

        fs::create_dir_all("docs")?;
        let md = tmp.join("docs").join("doc.md");
        let content = b"# Title\nContent\n";
        {
            let mut f = File::create(&md)?;
            f.write_all(content)?;
            f.sync_all()?;
        }

        let cache = PeisarCache::new("docs")?;
        let cache_file = tmp.join(".peisar_cache").join("docs").join("doc.md.json");
        assert!(cache_file.exists());

        let meta1 = fs::metadata(&cache_file)?.modified()?;
        // send update with same content
        if let Some(tx) = &cache.persist_tx {
            tx.send(PersistCommand::Update(
                md.clone(),
                CachedContent::Text(String::from_utf8_lossy(content).to_string()),
            ))
            .unwrap();
        }

        // Wait briefly to let worker run
        std::thread::sleep(std::time::Duration::from_millis(200));
        let meta2 = fs::metadata(&cache_file)?.modified()?;

        assert_eq!(meta1, meta2, "file was rewritten despite identical content");

        env::set_current_dir(orig)?;
        fs::remove_dir_all(&tmp)?;
        Ok(())
    }
}
