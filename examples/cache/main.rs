use peisar::cache::{CachedContent, PeisarCache, PeisarCacheConfig};
use std::path::Path;
fn main() -> std::io::Result<()> {
    let mut cache = PeisarCache::with_config(PeisarCacheConfig {
        entry_dir: ".notes/contents".into(),
        assets_dir: Some(".notes/public".into()),
        ..PeisarCacheConfig::default()
    })?;
    for path in cache.markdown_files() {
        if let Some(CachedContent::Text(md)) = cache.get(Path::new(&path)) {
            println!("{} ({} bytes)", path, md.len());
        }
    }
    cache.start_watching()?;
    Ok(())
}
