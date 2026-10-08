//! Size-aware LRU (least-recently-used) eviction for the in-memory cache.
//!
//! The cache stores raw [`CachedContent`] values keyed by absolute source
//! path. When the accumulated content size crosses the configured `budget`
//! (bytes), the least recently used entries are evicted until the cache is
//! back under it. Entries are also evicted when the *count* budget is hit.
//!
//! Access order is tracked with a monotonic epoch counter stamped on every
//! entry on `insert` / `get`, so entries actually being served stay
//! resident while cold ones fall out first.
//!
//! The store is deliberately *not* generic over the JS callback types: it
//! keeps raw file bytes/text only. Richer page objects (layout, title, html,
//! slug, …) are derived on demand from these entries and cached separately
//! in [`crate::cache::obj_cache`].

use crate::cache::CachedContent;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// LRU entry: content plus the bookkeeping needed for eviction.
struct LruEntry {
    content: CachedContent,
    last_used: u64,
}

/// Shared LRU store over absolute paths, bounded by bytes and entry count.
/// Cheap to clone: every clone shares the same underlying state, so the
/// file-watcher thread can update the store alongside the cache owner.
#[derive(Clone)]
pub struct LruCache {
    inner: Arc<Mutex<LruInner>>,
}

struct LruInner {
    map: HashMap<PathBuf, LruEntry>,
    /// Total size of all cached contents in bytes.
    total_bytes: u64,
    /// Maximum total content size in bytes (`0` = unlimited).
    byte_budget: u64,
    /// Maximum number of entries (`0` = unlimited).
    entry_budget: usize,
    /// Monotonic clock stamped on entries to track recency.
    epoch: u64,
    /// Number of entries evicted by the LRU policy so far (diagnostics).
    evicted_count: u64,
    /// Optional hook invoked with each evicted path. Runs *under the store
    /// lock*, so it must not call back into the cache.
    hook: Option<Box<dyn Fn(PathBuf) + Send + Sync>>,
}

impl LruCache {
    /// Create a store limited to `byte_budget` bytes of content and
    /// `entry_budget` entries (either may be `0` for "unlimited").
    pub fn new(byte_budget: u64, entry_budget: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LruInner {
                map: HashMap::new(),
                total_bytes: 0,
                byte_budget,
                entry_budget,
                epoch: 0,
                evicted_count: 0,
                hook: None,
            })),
        }
    }

    /// Register the eviction hook, invoked with the path of every entry
    /// evicted by the LRU policy (explicit `remove` does *not* trigger it —
    /// the caller controls invalidation for those). Runs under the store
    /// lock, so hooks must not call back into the cache (no re-entrancy).
    pub fn on_evict<F: Fn(PathBuf) + Send + Sync + 'static>(&self, f: F) {
        self.inner.lock().unwrap().hook = Some(Box::new(f));
    }

    /// Current total content size in bytes.
    pub fn size_bytes(&self) -> u64 {
        self.inner.lock().unwrap().total_bytes
    }

    /// Number of entries currently held.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().map.len()
    }

    /// `true` when no entries are held.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Total number of entries evicted by the LRU policy so far.
    pub fn evicted_count(&self) -> u64 {
        self.inner.lock().unwrap().evicted_count
    }

    /// Insert or replace an entry, stamping it as most recently used and
    /// evicting LRU entries while over budget.
    pub fn insert(&self, path: PathBuf, content: CachedContent) {
        let size = content_size_of(&content);
        let mut inner = self.inner.lock().unwrap();
        inner.epoch += 1;
        let tick = inner.epoch;
        // Replace: drop the old size accounting first.
        if let Some(old) = inner.map.insert(
            path,
            LruEntry {
                content,
                last_used: tick,
            },
        ) {
            inner.total_bytes = inner
                .total_bytes
                .saturating_sub(content_size_of(&old.content));
        }
        inner.total_bytes = inner.total_bytes.saturating_add(size);
        inner.evict_if_needed();
    }

    /// Insert or replace an entry *without* touching recency — used by the
    /// initial snapshot load, which should not promote any file over
    /// another.
    pub fn insert_cold(&self, path: PathBuf, content: CachedContent) {
        let size = content_size_of(&content);
        let mut inner = self.inner.lock().unwrap();
        if let Some(old) = inner.map.insert(
            path,
            LruEntry {
                content,
                last_used: 0,
            },
        ) {
            inner.total_bytes = inner
                .total_bytes
                .saturating_sub(content_size_of(&old.content));
        }
        inner.total_bytes = inner.total_bytes.saturating_add(size);
        inner.evict_if_needed();
    }

    /// Touch the recency of a path and return a clone of its content.
    pub fn get(&self, path: &Path) -> Option<CachedContent> {
        let mut inner = self.inner.lock().unwrap();
        inner.epoch += 1;
        let tick = inner.epoch;
        let entry = inner.map.get_mut(path)?;
        entry.last_used = tick;
        Some(entry.content.clone())
    }

    /// Remove a path, returning the content if present.
    /// The registered eviction hook is NOT invoked for explicit removals —
    /// only for LRU eviction, so the caller controls invalidation.
    pub fn remove(&self, path: &Path) -> Option<CachedContent> {
        let mut inner = self.inner.lock().unwrap();
        let entry = inner.map.remove(path)?;
        inner.total_bytes = inner
            .total_bytes
            .saturating_sub(content_size_of(&entry.content));
        Some(entry.content)
    }

    /// Replace the entire contents (used by the directory-rescan path).
    pub fn replace_all(&self, map: HashMap<PathBuf, CachedContent>) {
        let mut inner = self.inner.lock().unwrap();
        inner.map.clear();
        inner.total_bytes = 0;
        for (path, content) in map {
            let size = content_size_of(&content);
            inner.map.insert(
                path,
                LruEntry {
                    content,
                    last_used: 0,
                },
            );
            inner.total_bytes = inner.total_bytes.saturating_add(size);
        }
        inner.evict_if_needed();
    }

    /// Snapshot of all keys (paths are cloned out of the lock).
    pub fn keys(&self) -> Vec<PathBuf> {
        self.inner.lock().unwrap().map.keys().cloned().collect()
    }

    /// Snapshot of every `(path, content)` pair, in arbitrary order.
    pub fn iter_all(&self) -> Vec<(PathBuf, CachedContent)> {
        self.inner
            .lock()
            .unwrap()
            .map
            .iter()
            .map(|(p, e)| (p.clone(), e.content.clone()))
            .collect()
    }
}

impl LruInner {
    /// Evict least-recently-used entries while over budget. The eviction
    /// hook (if any) fires per evicted path *inside* this lock.
    fn evict_if_needed(&mut self) {
        while self.byte_budget > 0 && self.total_bytes > self.byte_budget && !self.map.is_empty() {
            self.evict_lru_once();
        }
        while self.entry_budget > 0 && self.map.len() > self.entry_budget {
            self.evict_lru_once();
        }
    }

    fn evict_lru_once(&mut self) {
        // Find the least recently used entry (smallest `last_used`).
        let victim = self
            .map
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(p, _)| p.clone());
        let Some(path) = victim else { return };
        if let Some(entry) = self.map.remove(&path) {
            self.total_bytes = self
                .total_bytes
                .saturating_sub(content_size_of(&entry.content));
            if let Some(hook) = &self.hook {
                hook(path);
            }
            self.evicted_count += 1;
        }
    }
}

/// Content size in bytes for accounting (string bytes / raw byte length).
pub fn content_size_of(content: &CachedContent) -> u64 {
    match content {
        CachedContent::Text(s) => s.len() as u64,
        CachedContent::Binary(b) => b.len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn text(s: &str) -> CachedContent {
        CachedContent::Text(s.to_string())
    }

    #[test]
    fn evicts_least_recently_used_when_over_byte_budget() {
        let cache = LruCache::new(10, 0);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_hook = Arc::clone(&seen);
        cache.on_evict(move |p| seen_hook.lock().unwrap().push(p));

        let a = PathBuf::from("/a.md"); // 4 bytes
        let b = PathBuf::from("/b.md"); // 6 bytes
        cache.insert(a.clone(), text("aaaa"));
        cache.insert(b.clone(), text("bbbbbb"));
        assert_eq!(cache.size_bytes(), 10);

        // Touch `a` so `b` becomes the LRU victim: with an 8-byte `c`
        // coming in (total 18), `b` (6 bytes) is evicted first (12
        // remaining), then `a` (4 bytes, stamp 2) still puts us at 8 —
        // both go, in recency order.
        cache.get(&a);
        cache.insert(PathBuf::from("/c.md"), text("cccccccc")); // 8 bytes
        assert_eq!(cache.size_bytes(), 8);
        assert!(cache.get(&b).is_none(), "b was the LRU victim");
        assert!(cache.get(&a).is_none(), "a was evicted second");
        assert!(cache.get(Path::new("/c.md")).is_some());

        let evicted = seen.lock().unwrap();
        assert_eq!(evicted.len(), 2);
        assert_eq!(evicted[0], PathBuf::from("/b.md"));
        assert_eq!(evicted[1], PathBuf::from("/a.md"));
    }

    #[test]
    fn evicts_by_entry_budget() {
        let cache = LruCache::new(0, 2);
        cache.insert(PathBuf::from("/1.md"), text("one"));
        cache.insert(PathBuf::from("/2.md"), text("two"));
        cache.insert(PathBuf::from("/3.md"), text("three"));
        assert_eq!(cache.len(), 2);
        assert!(cache.get(Path::new("/1.md")).is_none());
        assert!(cache.get(Path::new("/2.md")).is_some());
        assert_eq!(cache.evicted_count(), 1);
    }

    #[test]
    fn replace_all_resets_state() {
        let cache = LruCache::new(0, 3);
        cache.insert(PathBuf::from("/x.md"), text("xxxx"));
        let mut fresh = HashMap::new();
        fresh.insert(PathBuf::from("/y.md"), text("yy"));
        cache.replace_all(fresh);
        assert_eq!(cache.len(), 1);
        assert!(cache.get(Path::new("/x.md")).is_none());
        assert_eq!(cache.size_bytes(), 2);
    }

    #[test]
    fn unlimited_budgets_keep_everything() {
        let cache = LruCache::new(0, 0);
        for i in 0..50 {
            cache.insert(PathBuf::from(format!("/f{i}.md")), text("x"));
        }
        assert_eq!(cache.len(), 50);
        assert_eq!(cache.evicted_count(), 0);
    }

    #[test]
    fn insert_cold_is_evicted_before_warm_entries() {
        let cache = LruCache::new(0, 2);
        cache.insert_cold(PathBuf::from("/cold.md"), text("c"));
        cache.insert(PathBuf::from("/warm.md"), text("w"));
        // The cold entry has stamp 0, the warm one 1 → inserting a third
        // evicts the cold one.
        cache.insert(PathBuf::from("/new.md"), text("n"));
        assert!(cache.get(Path::new("/cold.md")).is_none());
        assert!(cache.get(Path::new("/warm.md")).is_some());
    }

    #[test]
    fn remove_does_not_fire_eviction_hook() {
        let cache = LruCache::new(10, 0);
        let evictions = Arc::new(Mutex::new(0u32));
        let evictions_hook = Arc::clone(&evictions);
        cache.on_evict(move |_| {
            *evictions_hook.lock().unwrap() += 1;
        });
        cache.insert(PathBuf::from("/a.md"), text("aaaa"));
        assert!(cache.remove(Path::new("/a.md")).is_some());
        assert_eq!(*evictions.lock().unwrap(), 0);
        assert_eq!(cache.evicted_count(), 0);
    }

    #[test]
    fn insert_replaces_previous_size_accounting() {
        let cache = LruCache::new(100, 0);
        cache.insert(PathBuf::from("/a.md"), text("short"));
        cache.insert(PathBuf::from("/a.md"), text("a much longer value"));
        assert_eq!(cache.size_bytes(), "a much longer value".len() as u64);
        assert_eq!(cache.len(), 1);
    }
}
