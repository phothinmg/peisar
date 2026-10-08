//! Tier-2 on-disk archive: memory-first caching with binary shards and a
//! JSON search index.
//!
//! The archive is a *cold tier* behind the in-memory LRU: it receives
//! upserts/removes on a background worker, buffers them, and — when either
//! the pending content crosses [`ArchiveHandle::new`]'s `shard_bytes`
//! threshold or the flush interval elapses — writes a new **shard** (a
//! compact bincode file) plus a rewritten **`map.json`** index:
//!
//! ```text
//! .peisar-cache/
//! ├── map.json          ← path → { shard, offset, length, kind, size, hash, frontmatter }
//! ├── shard-000001.bin  ← concatenated bincode entries
//! └── shard-000002.bin
//! ```
//!
//! `map.json` is the "search" index for the frontend: every markdown entry
//! carries its parsed front matter (title/tags/date), so a build step can
//! list pages without touching the shards. Reads go through the shared
//! in-memory index and read a single entry out of its shard by
//! `(offset, length)`.
//!
//! Shards are immutable once written: updated entries land in newer shards
//! and unreferenced old ones are garbage-collected on every flush. Writes
//! are atomic (temp file + rename), so a crash never leaves a torn shard
//! behind.

use crate::cache::CachedContent;
use crate::cache::lru::content_size_of;
use crate::cache::obj_cache::rel_to_base;
use crate::frontmatter::frontmatter;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// Directory name of the on-disk archive under the working directory.
pub const ARCHIVE_DIR_NAME: &str = ".peisar-cache";

/// Location + metadata of one archived entry.
#[derive(Debug, Clone)]
pub struct LocatedEntry {
    /// Shard file name (e.g. `shard-000001.bin`).
    pub shard: String,
    /// Byte offset of the entry inside the shard.
    pub offset: u64,
    /// Serialized entry length in bytes.
    pub length: u64,
    /// `"text"` for markdown/textual assets, `"binary"` otherwise.
    pub kind: &'static str,
    /// Raw content size in bytes.
    pub size: u64,
    /// BLAKE3 hash of the raw content (hex).
    pub hash: String,
    /// Parsed front matter for markdown entries (searchable metadata).
    pub frontmatter: Option<Value>,
}

/// Shared read-side index: absolute source path → archive location.
pub type ArchiveIndex = HashMap<PathBuf, LocatedEntry>;

/// Commands fed to the archive worker.
#[derive(Debug)]
pub(crate) enum ArchiveCommand {
    /// Insert or replace one entry.
    Upsert(PathBuf, CachedContent),
    /// Remove one entry (and its map entry).
    Remove(PathBuf),
    /// Replace the whole archive with this snapshot.
    SyncAll(HashMap<PathBuf, CachedContent>),
    /// Flush pending entries now; reply on the channel when done.
    FlushNow(SyncSender<()>),
}

/// Handle to the archive: the command channel, the shared read index, and
/// the worker join handle (shared). Cheap to clone: clones share the same
/// worker, so the file-watcher thread can feed upserts/removes through the
/// same handle the cache owner holds.
#[derive(Clone)]
pub struct ArchiveHandle {
    tx: Sender<ArchiveCommand>,
    index: Arc<Mutex<ArchiveIndex>>,
    dir: PathBuf,
    /// Shared so clones see close() having joined (and don't join twice);
    /// `None` for handles created by `map_json` rendering.
    worker: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ArchiveHandle {
    /// Spawn the archive worker writing shards under `<cwd>/.peisar-cache`.
    ///
    /// `shard_bytes` is the pending-content threshold that triggers a flush
    /// (`0` disables size-based flushing); `interval` is the maximum time
    /// pending entries may sit in memory before being flushed.
    pub fn new(cwd: PathBuf, shard_bytes: u64, interval: Duration) -> Self {
        let dir = cwd.join(ARCHIVE_DIR_NAME);
        let _ = fs::create_dir_all(&dir);

        let index: Arc<Mutex<ArchiveIndex>> = Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = std::sync::mpsc::channel::<ArchiveCommand>();

        let worker = std::thread::spawn({
            let dir = dir.clone();
            let index = Arc::clone(&index);
            move || {
                let mut state = Worker {
                    dir: dir.clone(),
                    shard_bytes,
                    interval,
                    pending: HashMap::new(),
                    pending_bytes: 0,
                    seq: next_shard_seq(&dir),
                    index,
                };
                state.run(rx);
            }
        });

        Self {
            tx,
            index,
            dir,
            worker: Arc::new(Mutex::new(Some(worker))),
        }
    }

    /// Insert or replace one entry (flushed later by size/interval).
    pub fn upsert(&self, path: PathBuf, content: CachedContent) {
        let _ = self.tx.send(ArchiveCommand::Upsert(path, content));
    }

    /// Remove one entry.
    pub fn remove(&self, path: PathBuf) {
        let _ = self.tx.send(ArchiveCommand::Remove(path));
    }

    /// Replace the whole archive with a fresh snapshot.
    pub fn sync_all(&self, map: HashMap<PathBuf, CachedContent>) {
        let _ = self.tx.send(ArchiveCommand::SyncAll(map));
    }

    /// Flush all pending entries now, blocking until the worker is done.
    pub fn flush(&self) {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        if self.tx.send(ArchiveCommand::FlushNow(tx)).is_ok() {
            let _ = rx.recv();
        }
    }

    /// Read one archived entry. Returns `None` for unknown paths or when
    /// the shard bytes fail to deserialize (a torn archive).
    pub fn read(&self, path: &Path) -> Option<CachedContent> {
        let (shard, offset, length) = {
            let index = self.index.lock().unwrap();
            let entry = index.get(path)?;
            (entry.shard.clone(), entry.offset, entry.length)
        };
        let mut file = fs::File::open(self.dir.join(&shard)).ok()?;
        let mut buf = vec![0u8; length as usize];
        file.seek(SeekFrom::Start(offset)).ok()?;
        file.read_exact(&mut buf).ok()?;
        bincode::deserialize::<CachedContent>(&buf).ok()
    }

    /// `true` when the path has been flushed to the archive. (Pending,
    /// not-yet-flushed entries return `false` — the memory tier still holds
    /// them.)
    pub fn contains(&self, path: &Path) -> bool {
        self.index.lock().unwrap().contains_key(path)
    }

    /// Number of archived (flushed) entries.
    pub fn entry_count(&self) -> usize {
        self.index.lock().unwrap().len()
    }

    /// Keys of the shared index (flushed entries), cloned out of the lock.
    pub fn keys(&self) -> Vec<PathBuf> {
        self.index.lock().unwrap().keys().cloned().collect()
    }

    /// Absolute archive directory (`<cwd>/.peisar-cache`).
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Serialize the current index as the `map.json` document: relative
    /// path → location + metadata. Entries still pending in the worker are
    /// not included until [`Self::flush`].
    pub fn map_json(&self) -> String {
        let index = self.index.lock().unwrap();
        let files: serde_json::Map<String, Value> = index
            .iter()
            .map(|(path, entry)| {
                let key = rel_to_base(path, &cwd_of(&self.dir))
                    .to_string_lossy()
                    .replace('\\', "/");
                (
                    key,
                    serde_json::to_value(MapFileEntry {
                        shard: &entry.shard,
                        offset: entry.offset,
                        length: entry.length,
                        kind: entry.kind,
                        size: entry.size,
                        hash: &entry.hash,
                        frontmatter: &entry.frontmatter,
                    })
                    .unwrap_or(Value::Null),
                )
            })
            .collect();
        serde_json::to_string_pretty(&MapFile { version: 1, files })
            .unwrap_or_else(|_| "{}".to_string())
    }

    /// Flush pending entries, stop the worker, and join it. Dropping the
    /// sender makes the worker flush its pending buffer, rewrite
    /// `map.json`, and exit. Safe to call more than once (later calls see
    /// the worker already taken and do nothing).
    pub fn close(&mut self) {
        // Drop our sender by replacing it with a closed channel.
        let (tx, _rx) = std::sync::mpsc::channel();
        self.tx = tx;
        let worker = self.worker.lock().unwrap().take();
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }
}

impl Drop for ArchiveHandle {
    fn drop(&mut self) {
        // Only the original owner closes: clones share the Arc so they see
        // the worker already taken (the Option is None after close()), and
        // a `None` worker means this handle never owned one.
        let has_worker = self.worker.lock().unwrap().is_some();
        if has_worker {
            self.close();
        }
    }
}

/// Recover the cwd from the archive dir (it is always `<cwd>/.peisar-cache`).
fn cwd_of(dir: &Path) -> PathBuf {
    dir.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| dir.to_path_buf())
}

/// On-disk `map.json` shape.
#[derive(Serialize)]
struct MapFile {
    version: u32,
    files: serde_json::Map<String, Value>,
}

/// One entry value inside `map.json`.
#[derive(Serialize)]
struct MapFileEntry<'a> {
    shard: &'a str,
    offset: u64,
    length: u64,
    kind: &'a str,
    size: u64,
    hash: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    frontmatter: &'a Option<Value>,
}

/// The archive worker: buffers commands, writes shards + `map.json`.
struct Worker {
    dir: PathBuf,
    shard_bytes: u64,
    interval: Duration,
    pending: HashMap<PathBuf, CachedContent>,
    pending_bytes: u64,
    seq: u64,
    index: Arc<Mutex<ArchiveIndex>>,
}

impl Worker {
    fn run(&mut self, rx: std::sync::mpsc::Receiver<ArchiveCommand>) {
        loop {
            match rx.recv_timeout(self.interval) {
                Ok(cmd) => self.handle(cmd),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if !self.pending.is_empty() {
                        self.flush();
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    self.flush();
                    break;
                }
            }
        }
    }

    fn handle(&mut self, cmd: ArchiveCommand) {
        match cmd {
            ArchiveCommand::Upsert(path, content) => {
                self.upsert_pending(path, content);
                if self.shard_bytes > 0 && self.pending_bytes >= self.shard_bytes {
                    self.flush();
                }
            }
            ArchiveCommand::Remove(path) => {
                self.pending.remove(&path);
                self.index.lock().unwrap().remove(&path);
            }
            ArchiveCommand::SyncAll(map) => {
                self.pending = map;
                self.pending_bytes = self.pending.values().map(content_size_of).sum();
                let live = self.pending.keys().cloned().collect::<Vec<_>>();
                self.index.lock().unwrap().retain(|p, _| live.contains(p));
                if self.shard_bytes > 0 && self.pending_bytes >= self.shard_bytes {
                    self.flush();
                }
            }
            ArchiveCommand::FlushNow(reply) => {
                self.flush();
                let _ = reply.send(());
            }
        }
    }

    fn upsert_pending(&mut self, path: PathBuf, content: CachedContent) {
        let size = content_size_of(&content);
        if let Some(old) = self.pending.insert(path, content) {
            self.pending_bytes = self.pending_bytes.saturating_sub(content_size_of(&old));
        }
        self.pending_bytes = self.pending_bytes.saturating_add(size);
    }

    /// Write every pending entry into new shard(s), update the shared
    /// index, rewrite `map.json`, and garbage-collect orphaned shards.
    fn flush(&mut self) {
        if self.pending.is_empty() {
            self.write_map_json();
            self.gc_shards();
            return;
        }
        let mut entries: Vec<(PathBuf, CachedContent)> =
            std::mem::take(&mut self.pending).into_iter().collect();
        self.pending_bytes = 0;
        // Deterministic shard contents.
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let mut chunk: Vec<(PathBuf, CachedContent)> = Vec::new();
        let mut chunk_bytes = 0u64;
        for entry in entries {
            let size = content_size_of(&entry.1);
            chunk_bytes = chunk_bytes.saturating_add(size);
            chunk.push(entry);
            if self.shard_bytes > 0 && chunk_bytes >= self.shard_bytes {
                chunk_bytes = 0;
                self.write_shard(std::mem::take(&mut chunk));
            }
        }
        if !chunk.is_empty() {
            self.write_shard(chunk);
        }

        self.write_map_json();
        self.gc_shards();
    }

    /// Write one shard file with the given entries and register their
    /// locations in the shared index.
    fn write_shard(&mut self, entries: Vec<(PathBuf, CachedContent)>) {
        self.seq += 1;
        let shard_name = format!("shard-{seq:06}.bin", seq = self.seq);
        let tmp_name = format!("{shard_name}.tmp");

        let mut located: Vec<(PathBuf, LocatedEntry)> = Vec::with_capacity(entries.len());
        let mut buf: Vec<u8> = Vec::new();
        let mut offset = 0u64;

        for (path, content) in entries {
            let bytes = bincode::serialize(&content).unwrap_or_default();
            if bytes.is_empty() {
                continue;
            }
            let kind = match content {
                CachedContent::Text(_) => "text",
                CachedContent::Binary(_) => "binary",
            };
            let (hash, frontmatter) = match &content {
                CachedContent::Text(s) => (
                    blake3::hash(s.as_bytes()).to_hex().to_string(),
                    match frontmatter(s.clone()) {
                        Ok(parsed) => parsed.yaml_data().cloned(),
                        Err(_) => None,
                    },
                ),
                CachedContent::Binary(b) => (blake3::hash(b).to_hex().to_string(), None),
            };
            let length = bytes.len() as u64;
            let size = content_size_of(&content);
            buf.extend_from_slice(&bytes);
            located.push((
                path,
                LocatedEntry {
                    shard: shard_name.clone(),
                    offset,
                    length,
                    kind,
                    size,
                    hash,
                    frontmatter,
                },
            ));
            offset += length;
        }

        // Atomic shard write: temp file + rename.
        if let Ok(mut tmp) = fs::File::create(self.dir.join(&tmp_name)) {
            if tmp.write_all(&buf).is_ok() {
                let _ = fs::rename(self.dir.join(&tmp_name), self.dir.join(&shard_name));
            } else {
                let _ = fs::remove_file(self.dir.join(&tmp_name));
            }
        }

        let mut index = self.index.lock().unwrap();
        for (path, entry) in located {
            index.insert(path, entry);
        }
    }

    /// Rewrite `map.json` from the shared index (atomic tmp + rename).
    fn write_map_json(&self) {
        let handle = ArchiveHandle {
            tx: {
                let (tx, _rx) = std::sync::mpsc::channel();
                tx
            },
            index: Arc::clone(&self.index),
            dir: self.dir.clone(),
            worker: Arc::new(Mutex::new(None)),
        };
        let json = handle.map_json();
        let tmp = self.dir.join("map.json.tmp");
        if fs::write(&tmp, json.as_bytes()).is_ok() {
            let _ = fs::rename(&tmp, self.dir.join("map.json"));
        }
    }

    /// Delete shard files no longer referenced by the index.
    fn gc_shards(&self) {
        let referenced: std::collections::HashSet<String> = self
            .index
            .lock()
            .unwrap()
            .values()
            .map(|e| e.shard.clone())
            .collect();
        let entries = match fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with("shard-") && name.ends_with(".bin") && !referenced.contains(name) {
                let _ = fs::remove_file(&p);
            }
        }
    }
}

/// First free shard sequence number for a directory (existing shards may be
/// left over from a previous run; we never overwrite them).
fn next_shard_seq(dir: &Path) -> u64 {
    let mut max = 0u64;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(num) = name
                .strip_prefix("shard-")
                .and_then(|s| s.strip_suffix(".bin"))
                .and_then(|s| s.parse::<u64>().ok())
            {
                max = max.max(num);
            }
        }
    }
    max
}

/// Ensure `.peisar-cache` is present in `.gitignore` (creating the file
/// when missing). Called synchronously at cache construction.
pub(crate) fn ensure_gitignore_entry(cwd: &Path) -> io::Result<()> {
    use std::fs::OpenOptions;
    let gitignore_path = cwd.join(".gitignore");
    if gitignore_path.exists() {
        let existing = fs::read_to_string(&gitignore_path)?;
        let mut has_entry = false;
        for line in existing.lines() {
            if line.trim() == ARCHIVE_DIR_NAME {
                has_entry = true;
                break;
            }
        }
        if !has_entry {
            let mut file = OpenOptions::new().append(true).open(&gitignore_path)?;
            if !existing.ends_with('\n') {
                file.write_all(b"\n")?;
            }
            file.write_all(format!("{ARCHIVE_DIR_NAME}\n").as_bytes())?;
        }
    } else {
        fs::write(gitignore_path, format!("{ARCHIVE_DIR_NAME}\n"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration as D;

    fn tmp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "peisar_archive_{tag}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn upsert_flush_and_read_back() {
        let cwd = tmp_dir("roundtrip");
        let mut archive = ArchiveHandle::new(cwd.clone(), 1024, D::from_millis(60_000));
        let md = cwd.join("docs").join("page.md");
        archive.upsert(
            md.clone(),
            CachedContent::Text("---\ntitle: Hello\n---\n\n# Hello\n".to_string()),
        );
        archive.flush();

        assert!(archive.contains(&md));
        match archive.read(&md) {
            Some(CachedContent::Text(text)) => assert!(text.contains("# Hello")),
            other => panic!("expected text, got {other:?}"),
        }

        // map.json on disk, keyed by the relative path, with front matter.
        let map = fs::read_to_string(cwd.join(ARCHIVE_DIR_NAME).join("map.json")).unwrap();
        let map: Value = serde_json::from_str(&map).unwrap();
        assert_eq!(map["version"], 1);
        let entry = &map["files"]["docs/page.md"];
        assert_eq!(entry["kind"], "text");
        assert_eq!(entry["frontmatter"]["title"], "Hello");
        assert!(entry["hash"].as_str().unwrap().len() == 64);

        archive.close();
        let _ = fs::remove_dir_all(&cwd);
    }

    #[test]
    fn binary_entries_roundtrip() {
        let cwd = tmp_dir("binary");
        let mut archive = ArchiveHandle::new(cwd.clone(), 0, D::from_millis(60_000));
        let png = cwd.join("logo.png");
        let bytes = vec![1u8, 2, 3, 4];
        archive.upsert(png.clone(), CachedContent::Binary(bytes.clone()));
        archive.flush();
        match archive.read(&png) {
            Some(CachedContent::Binary(b)) => assert_eq!(b, bytes),
            other => panic!("expected binary, got {other:?}"),
        }
        archive.close();
        let _ = fs::remove_dir_all(&cwd);
    }

    #[test]
    fn remove_drops_entry_and_gc_collects_orphan_shard() {
        let cwd = tmp_dir("remove");
        let mut archive = ArchiveHandle::new(cwd.clone(), 0, D::from_millis(60_000));
        let a = cwd.join("a.md");
        archive.upsert(a.clone(), CachedContent::Text("A".to_string()));
        archive.flush();
        assert!(archive.contains(&a));

        archive.remove(a.clone());
        archive.flush();
        assert!(!archive.contains(&a));
        assert!(archive.read(&a).is_none());

        // map.json no longer references the file.
        let map = fs::read_to_string(cwd.join(ARCHIVE_DIR_NAME).join("map.json")).unwrap();
        let map: Value = serde_json::from_str(&map).unwrap();
        assert!(map["files"].get("a.md").is_none());

        // The shard that held only `a` is garbage-collected.
        let shards: Vec<_> = fs::read_dir(cwd.join(ARCHIVE_DIR_NAME))
            .unwrap()
            .flatten()
            .filter(|e| {
                let n = e.file_name();
                let n = n.to_string_lossy();
                n.starts_with("shard-") && n.ends_with(".bin")
            })
            .collect();
        assert!(shards.is_empty(), "orphan shards should be GC'd");

        archive.close();
        let _ = fs::remove_dir_all(&cwd);
    }

    #[test]
    fn sync_all_replaces_everything() {
        let cwd = tmp_dir("sync");
        let mut archive = ArchiveHandle::new(cwd.clone(), 0, D::from_millis(60_000));
        archive.upsert(cwd.join("old.md"), CachedContent::Text("old".to_string()));
        archive.flush();
        assert!(archive.entry_count() == 1);

        let mut fresh = HashMap::new();
        fresh.insert(cwd.join("new.md"), CachedContent::Text("new".to_string()));
        archive.sync_all(fresh);
        archive.flush();
        assert!(!archive.contains(&cwd.join("old.md")));
        assert!(archive.contains(&cwd.join("new.md")));

        archive.close();
        let _ = fs::remove_dir_all(&cwd);
    }

    #[test]
    fn interval_flushes_without_size_threshold() {
        let cwd = tmp_dir("interval");
        // Zero shard threshold → only the interval flushes.
        let mut archive = ArchiveHandle::new(cwd.clone(), 0, D::from_millis(50));
        archive.upsert(cwd.join("lazy.md"), CachedContent::Text("lazy".to_string()));
        // Not flushed yet.
        assert!(!archive.contains(&cwd.join("lazy.md")));
        // Wait past the interval.
        thread::sleep(D::from_millis(250));
        assert!(archive.contains(&cwd.join("lazy.md")));

        archive.close();
        let _ = fs::remove_dir_all(&cwd);
    }

    #[test]
    fn drop_flushes_pending_entries() {
        let cwd = tmp_dir("drop");
        let md = cwd.join("gone.md");
        {
            let archive = ArchiveHandle::new(cwd.clone(), 0, D::from_millis(60_000));
            archive.upsert(md.clone(), CachedContent::Text("bye".to_string()));
            // no explicit flush; Drop must persist it
        }
        let map = fs::read_to_string(cwd.join(ARCHIVE_DIR_NAME).join("map.json")).unwrap();
        assert!(map.contains("gone.md"));
        let _ = fs::remove_dir_all(&cwd);
    }

    #[test]
    fn gitignore_entry_added_once() {
        let cwd = tmp_dir("gitignore");
        fs::write(cwd.join(".gitignore"), "target\n").unwrap();
        ensure_gitignore_entry(&cwd).unwrap();
        ensure_gitignore_entry(&cwd).unwrap();
        let git = fs::read_to_string(cwd.join(".gitignore")).unwrap();
        assert_eq!(
            git.lines().filter(|l| l.trim() == ARCHIVE_DIR_NAME).count(),
            1
        );

        // missing file is created
        let fresh = tmp_dir("gitignore_fresh");
        ensure_gitignore_entry(&fresh).unwrap();
        let git = fs::read_to_string(fresh.join(".gitignore")).unwrap();
        assert_eq!(git.trim(), ARCHIVE_DIR_NAME);
        let _ = fs::remove_dir_all(&cwd);
        let _ = fs::remove_dir_all(&fresh);
    }
}
