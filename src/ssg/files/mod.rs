//! File-discovery helpers for the SSG: markdown and asset collection.
//!
//! [`collect_markdown_files`] and [`collect_asset_files`] walk an entry
//! directory recursively and return absolute paths (resolved against the
//! process working directory) whose extensions appear in
//! [`MARKDOWN_EXTENSIONS`] / [`ASSET_EXTENSIONS`].
//!
//! The extension lists follow the community conventions documented at
//! <https://github.com/sindresorhus/markdown-extensions> (markdown) plus a
//! curated set of image / script / style / font / media extensions (assets).
#![allow(unused)]
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Recognized Markdown file extensions (lowercase, no leading dot).
///
/// Source: <https://github.com/sindresorhus/markdown-extensions>
pub const MARKDOWN_EXTENSIONS: &[&str] = &[
    "md", "markdown", "mdown", "mkdn", "mkd", "mdwn", "mkdown", "ron",
];

/// Recognized asset file extensions (images, scripts, styles, fonts,
/// audio/video), lowercase without leading dot.
pub const ASSET_EXTENSIONS: &[&str] = &[
    // images
    "png", "jpg", "jpeg", "gif", "svg", "webp", "avif", "ico", "bmp", "tiff",
    // scripts & styles
    "js", "css", "map", // fonts
    "woff", "woff2", "ttf", "eot", // audio/video
    "mp3", "wav", "ogg", "mp4", "webm",
];

/// Asset extensions whose contents must be read as raw bytes rather than
/// UTF-8 text.  Everything in this list is a subset of [`ASSET_EXTENSIONS`].
pub const ASSET_BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "avif", "ico", "bmp", "tiff", "woff", "woff2", "ttf",
    "eot", "mp3", "wav", "ogg", "mp4", "webm",
];

/// Recursively collect every file under `entry_dir` whose extension is in
/// [`MARKDOWN_EXTENSIONS`].
///
/// `entry_dir` may be absolute or relative to the current working
/// directory; returned paths are absolute.  A non-existent directory
/// yields an empty list (no error).
pub fn collect_markdown_files<P: AsRef<Path>>(entry_dir: P) -> io::Result<Vec<PathBuf>> {
    let current_dir = env::current_dir().unwrap_or(PathBuf::from("."));
    let dir_path = current_dir.join(entry_dir.as_ref());
    let mut markdown_files: Vec<PathBuf> = Vec::new();

    if dir_path.is_dir() {
        for entry in fs::read_dir(&dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recurse into subdirectory; PathBuf implements AsRef<Path>
                let nested = collect_markdown_files(path)?;
                markdown_files.extend(nested);
            } else if path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| MARKDOWN_EXTENSIONS.contains(&ext))
            {
                markdown_files.push(path);
            }
        }
    }

    Ok(markdown_files)
}

/// Recursively collect every file under `entry_dir` whose extension is in
/// [`ASSET_EXTENSIONS`].
///
/// `entry_dir` may be absolute or relative to the current working
/// directory; returned paths are absolute.  A non-existent directory
/// yields an empty list (no error).
pub fn collect_asset_files<P: AsRef<Path>>(entry_dir: P) -> io::Result<Vec<PathBuf>> {
    let current_dir = env::current_dir().unwrap_or(PathBuf::from("."));
    let dir_path = current_dir.join(entry_dir.as_ref());
    let mut asset_files: Vec<PathBuf> = Vec::new();

    if dir_path.is_dir() {
        for entry in fs::read_dir(&dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recurse
                let nested = collect_asset_files(path)?;
                asset_files.extend(nested);
            } else if path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| ASSET_EXTENSIONS.contains(&ext))
            {
                asset_files.push(path);
            }
        }
    }

    Ok(asset_files)
}

#[cfg(test)]
mod tests {
    use super::{
        ASSET_BINARY_EXTENSIONS, ASSET_EXTENSIONS, MARKDOWN_EXTENSIONS, collect_asset_files,
        collect_markdown_files,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    /// A unique temporary directory removed recursively on drop.  Tests that
    /// resolve relative entry dirs must hold the cwd lock because the helpers
    /// resolve against the process working directory.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("peisar_files-{}-{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn write(&self, rel: &str) -> PathBuf {
            let p = self.0.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, "x").unwrap();
            p
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn binary_extensions_are_a_subset_of_asset_extensions() {
        for ext in ASSET_BINARY_EXTENSIONS {
            assert!(
                ASSET_EXTENSIONS.contains(ext),
                "{ext} is binary but missing from ASSET_EXTENSIONS"
            );
        }
    }

    #[test]
    fn collect_markdown_files_finds_nested_files() {
        let tmp = TempDir::new("md");
        tmp.write("a.md");
        tmp.write("sub/b.markdown");
        tmp.write("sub/deep/c.mkd");
        // Non-markdown extensions are ignored.
        tmp.write("d.txt");
        tmp.write("e.css");

        let files = collect_markdown_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 3);
        assert!(files.iter().all(|p| p.is_absolute()));
        assert!(files.iter().any(|p| p.ends_with("a.md")));
        assert!(files.iter().any(|p| p.ends_with("b.markdown")));
        assert!(files.iter().any(|p| p.ends_with("c.mkd")));
    }

    #[test]
    fn collect_asset_files_finds_supported_types() {
        let tmp = TempDir::new("assets");
        tmp.write("img/logo.png");
        tmp.write("style/main.css");
        tmp.write("app.js");
        // Markdown files are not assets.
        tmp.write("note.md");

        let files = collect_asset_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|p| p.ends_with("logo.png")));
        assert!(files.iter().any(|p| p.ends_with("main.css")));
        assert!(files.iter().any(|p| p.ends_with("app.js")));
    }

    #[test]
    fn missing_directory_yields_empty_list() {
        let missing = std::env::temp_dir().join("peisar_files-definitely-missing");
        assert!(collect_markdown_files(&missing).unwrap().is_empty());
        assert!(collect_asset_files(&missing).unwrap().is_empty());
    }

    #[test]
    fn markdown_extensions_match_the_upstream_list() {
        // Guard against accidental edits to the canonical extension list.
        assert_eq!(
            MARKDOWN_EXTENSIONS,
            &[
                "md", "markdown", "mdown", "mkdn", "mkd", "mdwn", "mkdown", "ron"
            ]
        );
    }
}
