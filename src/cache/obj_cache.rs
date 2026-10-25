//! Template-ready objects for the site's pages and assets.
//!
//! [`PageObject`] / [`AssetsObject`] are the shapes a Liquid theme (Jekyll
//! style) renders with: plain string/array fields so templates can use
//! `{{ page.title }}` / `{{ page.layout }}` directly. Both cross the NAPI
//! boundary as napi objects.
//!
//! The objects are derived from the raw cache entries on demand and cached
//! in an [`ObjectCache`]: a small key → array store with in-place array
//! operations (push / remove / replace / update) so the watcher can keep
//! derived objects consistent without rebuilding whole arrays.

#[cfg(feature = "npm")]
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// A page rendered for a Liquid theme: front-matter fields flattened next
/// to pipeline-derived fields (layout, slug, html, …).
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageObject {
    /// Layout name from front matter (e.g. `default`, `page`, `post`);
    /// `"default"` when front matter does not set one.
    pub layout: String,
    /// File name without extension (`foo.md` → `foo`); `index` for
    /// directory roots. Used as a search / lookup key.
    pub file_name: String,
    /// Request URL for the page (`path/to/foo.md` → `/path/to/foo`).
    pub slug: String,
    /// HTML rendered from the markdown by Peisar.
    pub html: String,
    /// Build output path (`path/to/foo.md` → `outDir/path/to/foo/index.html`),
    /// relative to the output root unless absolute.
    pub out_file_path: String,
    /// Original absolute source path (watch-process identity).
    pub input_file_path: String,
    /// Page title (from front matter, or the first `#` heading).
    pub title: Option<String>,
    /// Short description from front matter.
    pub summary: Option<String>,
    /// Category tags from front matter.
    pub tags: Option<Vec<String>>,
    /// Publication date from front matter (string form `yyyy-mm-dd`).
    pub publish_date: Option<String>,
    /// Any extra front-matter keys not mapped to a named field, exposed
    /// verbatim (as strings) so themes can render `{{ page.custom }}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, String>>,
}

/// An asset mirrored by the cache (JS/CSS/image/video/font/…).
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetsObject {
    /// File name without extension (`foo.png` → `foo`).
    pub file_name: String,
    /// URL for the asset (`path/to/foo.png` → `/path/to/foo.png`).
    pub slug: String,
    /// Raw content: UTF-8 text for textual assets, base64 for binaries.
    pub raw: String,
    /// Build output path for the asset.
    pub out_file_path: String,
    /// Original absolute source path (watch-process identity).
    pub input_file_path: String,
    /// File extension (lowercase, without dot).
    pub ext: String,
    /// Kind classification: `"Js"`, `"Css"`, `"Image"`, `"Video"`,
    /// `"Font"`, or `"Other"`.
    pub file_type: String,
}

/// Classify an asset extension into the `fileType` buckets.
pub fn asset_file_type(ext: &str) -> &'static str {
    match ext {
        "js" | "mjs" | "cjs" => "Js",
        "css" => "Css",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "avif" | "ico" | "bmp" | "tiff" => {
            "Image"
        }
        "mp3" | "wav" | "ogg" | "mp4" | "webm" => "Video",
        "woff" | "woff2" | "ttf" | "eot" => "Font",
        _ => "Other",
    }
}

/// Strip the extension from a file name (`foo.md` → `foo`; a leading-dot
/// file like `.hidden` keeps its name).
pub fn file_name_no_ext(file_name: &str) -> String {
    match file_name.rfind('.') {
        Some(0) => file_name.to_string(),
        Some(i) => file_name[..i].to_string(),
        None => file_name.to_string(),
    }
}

/// Compute the URL slug for a markdown file relative to a base directory,
/// with pretty URLs: `index` collapses to the directory root
/// (`dir/index.md` → `dir/`). `base_url` (e.g. `"/"` or `"/repo"`) is
/// prefixed and normalized.
pub fn markdown_slug(path: &Path, base_dir: &Path, base_url: &str) -> String {
    let rel = rel_to_base(path, base_dir);
    let mut s = rel;
    s.set_extension("");
    let mut slug = s.to_string_lossy().replace('\\', "/");
    if slug == "index" {
        slug = String::new();
    } else if let Some(stripped) = slug.strip_suffix("/index") {
        slug = stripped.to_string();
    }
    if slug.is_empty() {
        base_url.trim_end_matches('/').to_string() + "/"
    } else {
        format!("{}/{}", base_url.trim_end_matches('/'), slug)
    }
}

/// Compute the output file for a page: pretty URLs — `path/to/foo.md` →
/// `path/to/foo/index.html`; `index.md` becomes `index.html` directly.
pub fn page_out_file(path: &Path, base_dir: &Path, out_dir: &str) -> String {
    let rel = rel_to_base(path, base_dir);
    let mut rel = rel;
    rel.set_extension("html");
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    let root = out_dir.trim_end_matches('/');
    if rel_str == "index.html" {
        format!("{root}/index.html")
    } else if let Some(stripped) = rel_str.strip_suffix(".html") {
        format!("{root}/{stripped}/index.html")
    } else {
        format!("{root}/{rel_str}")
    }
}

/// Compute the URL slug for an asset (keeps its extension).
pub fn asset_slug(path: &Path, base_dir: &Path, base_url: &str) -> String {
    let rel = rel_to_base(path, base_dir)
        .to_string_lossy()
        .replace('\\', "/");
    format!("{}/{}", base_url.trim_end_matches('/'), rel)
}

/// Compute the output path for an asset: `path/to/foo.png` →
/// `outDir/path/to/foo.png`.
pub fn asset_out_file(path: &Path, base_dir: &Path, out_dir: &str) -> String {
    let rel = rel_to_base(path, base_dir)
        .to_string_lossy()
        .replace('\\', "/");
    format!("{}/{}", out_dir.trim_end_matches('/'), rel)
}

/// Path relative to `base_dir`, falling back to the full path with leading
/// root stripped when the file lives outside the base.
pub(crate) fn rel_to_base(path: &Path, base_dir: &Path) -> PathBuf {
    match path.strip_prefix(base_dir) {
        Ok(rel) => rel.to_path_buf(),
        Err(_) => {
            let mut rel = PathBuf::new();
            rel.extend(path.components().filter_map(|c| match c {
                std::path::Component::Normal(p) => Some(p),
                _ => None,
            }));
            rel
        }
    }
}

/// Build the `extra` map: front-matter keys not covered by the named
/// [`PageObject`] fields, stringified so Liquid templates can render them.
pub fn extra_frontmatter_fields(
    frontmatter: &serde_json::Value,
) -> Option<HashMap<String, String>> {
    let map = frontmatter.as_object()?;
    let named = [
        "layout",
        "title",
        "summary",
        "tags",
        "date",
        "publish_date",
        "publishDate",
    ];
    let mut extra: HashMap<String, String> = HashMap::new();
    for (k, v) in map {
        if named.contains(&k.as_str()) {
            continue;
        }
        let rendered = match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        extra.insert(k.clone(), rendered);
    }
    if extra.is_empty() { None } else { Some(extra) }
}

/// Key → array-of-objects store with in-place array operations, used to
/// keep the derived page/asset objects in sync without cloning whole arrays
/// on every watcher tick.
#[derive(Clone, Default)]
pub struct ObjectCache<V> {
    inner: Arc<Mutex<HashMap<String, Vec<V>>>>,
}

impl<V: Clone> ObjectCache<V> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Replace the array stored for a key, returning the old array.
    pub fn insert(&self, key: String, value: Vec<V>) -> Option<Vec<V>> {
        self.inner.lock().unwrap().insert(key, value)
    }

    /// Clone of the array stored for a key.
    pub fn get(&self, key: &str) -> Option<Vec<V>> {
        self.inner.lock().unwrap().get(key).cloned()
    }

    /// Remove a key and its array.
    pub fn remove(&self, key: &str) -> Option<Vec<V>> {
        self.inner.lock().unwrap().remove(key)
    }

    /// `true` when something is stored under the key.
    pub fn contains(&self, key: &str) -> bool {
        self.inner.lock().unwrap().contains_key(key)
    }

    // --- array operations inside a single lock ---

    /// Push an object to the end of the array for `key`. Returns `false`
    /// when the key does not exist yet.
    pub fn push_item(&self, key: &str, item: V) -> bool {
        let mut map = self.inner.lock().unwrap();
        if let Some(array) = map.get_mut(key) {
            array.push(item);
            true
        } else {
            false // Key didn't exist
        }
    }

    /// Removes the item at `index` from the array for `key`.
    pub fn remove_item_at(&self, key: &str, index: usize) -> Option<V> {
        let mut map = self.inner.lock().unwrap();
        if let Some(array) = map.get_mut(key)
            && index < array.len()
        {
            return Some(array.remove(index));
        }
        None
    }

    /// Replace the item at `index` for `key` with `new_item`.
    pub fn replace_item_at(&self, key: &str, index: usize, new_item: V) -> bool {
        let mut map = self.inner.lock().unwrap();
        if let Some(array) = map.get_mut(key)
            && index < array.len()
        {
            array[index] = new_item;
            return true;
        }
        false
    }

    /// Find an item matching `predicate` and replace it with `new_item`.
    /// Returns `true` when found and replaced.
    pub fn replace_item_by<F>(&self, key: &str, predicate: F, new_item: V) -> bool
    where
        F: Fn(&V) -> bool,
    {
        let mut map = self.inner.lock().unwrap();

        if let Some(array) = map.get_mut(key) {
            // Find the index of the first item that matches the condition.
            if let Some(index) = array.iter().position(predicate) {
                array[index] = new_item;
                return true;
            }
        }
        false
    }

    /// Gives a closure mutable access to the whole array for `key`, so
    /// callers can modify specific fields in place.
    pub fn update_items<F>(&self, key: &str, mut update_fn: F) -> bool
    where
        F: FnMut(&mut [V]),
    {
        let mut map = self.inner.lock().unwrap();
        if let Some(array) = map.get_mut(key) {
            update_fn(array);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_type_classification() {
        assert_eq!(asset_file_type("png"), "Image");
        assert_eq!(asset_file_type("js"), "Js");
        assert_eq!(asset_file_type("css"), "Css");
        assert_eq!(asset_file_type("woff2"), "Font");
        assert_eq!(asset_file_type("mp4"), "Video");
        assert_eq!(asset_file_type("map"), "Other");
    }

    #[test]
    fn strips_extensions_from_file_names() {
        assert_eq!(file_name_no_ext("foo.md"), "foo");
        assert_eq!(file_name_no_ext("foo.bar.md"), "foo.bar");
        assert_eq!(file_name_no_ext("noext"), "noext");
        assert_eq!(file_name_no_ext(".hidden"), ".hidden");
    }

    #[test]
    fn slugs_collapse_index_pages() {
        let base = PathBuf::from("/site/contents");
        let root_index = PathBuf::from("/site/contents/index.md");
        let nested = PathBuf::from("/site/contents/docs/intro.md");
        let nested_index = PathBuf::from("/site/contents/docs/index.md");
        assert_eq!(markdown_slug(&root_index, &base, "/"), "/");
        assert_eq!(markdown_slug(&nested, &base, ""), "/docs/intro");
        assert_eq!(markdown_slug(&nested_index, &base, "/blog"), "/blog/docs");
    }

    #[test]
    fn out_files_use_pretty_urls() {
        let base = PathBuf::from("/site/contents");
        let root_index = PathBuf::from("/site/contents/index.md");
        let nested = PathBuf::from("/site/contents/docs/intro.md");
        assert_eq!(page_out_file(&root_index, &base, "out"), "out/index.html");
        assert_eq!(
            page_out_file(&nested, &base, "out"),
            "out/docs/intro/index.html"
        );
    }

    #[test]
    fn asset_paths_keep_extension() {
        let base = PathBuf::from("/site/public");
        let img = PathBuf::from("/site/public/img/logo.png");
        assert_eq!(asset_slug(&img, &base, "/"), "/img/logo.png");
        assert_eq!(asset_out_file(&img, &base, "dist"), "dist/img/logo.png");
    }

    #[test]
    fn extra_fields_skip_named_keys() {
        let fm = serde_json::json!({
            "title": "T",
            "author": "someone",
            "weight": 3,
        });
        let extra = extra_frontmatter_fields(&fm).unwrap();
        assert_eq!(extra["author"], "someone");
        assert_eq!(extra["weight"], "3");
        assert!(!extra.contains_key("title"));
        let only_named = serde_json::json!({ "title": "T" });
        assert!(extra_frontmatter_fields(&only_named).is_none());
    }

    #[test]
    fn object_cache_array_ops() {
        let cache: ObjectCache<u32> = ObjectCache::new();
        assert!(!cache.push_item("k", 1)); // key missing
        cache.insert("k".into(), vec![10, 20]);
        assert!(cache.push_item("k", 30));
        assert_eq!(cache.get("k"), Some(vec![10, 20, 30]));
        assert_eq!(cache.remove_item_at("k", 0), Some(10));
        assert!(cache.replace_item_at("k", 0, 99));
        assert_eq!(cache.get("k"), Some(vec![99, 30]));
        assert!(cache.replace_item_by("k", |v| *v == 30, 33));
        assert_eq!(cache.get("k"), Some(vec![99, 33]));
        assert!(cache.update_items("k", |arr| { arr.iter_mut().for_each(|v| *v += 1) }));
        assert_eq!(cache.get("k"), Some(vec![100, 34]));
        assert!(cache.contains("k"));
        assert_eq!(cache.remove("k"), Some(vec![100, 34]));
        assert!(!cache.contains("k"));
    }
}
