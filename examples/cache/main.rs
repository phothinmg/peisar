use peisar::cache::{PeisarCache, PeisarCacheConfig};

/// Set a flag when SIGINT (Ctrl+C) arrives, so the main loop can stop
/// watching and exit gracefully.
static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

extern "C" fn on_sigint(_sig: libc::c_int) {
    STOP.store(true, std::sync::atomic::Ordering::SeqCst);
}

fn main() -> std::io::Result<()> {
    let mut cache = PeisarCache::with_config(PeisarCacheConfig {
        entry_dir: ".notes/contents".into(),
        assets_dir: Some(".notes/public".into()),
        ..PeisarCacheConfig::default()
    })?;
    // for path in cache.markdown_files() {
    //     if let Some(CachedContent::Text(md)) = cache.get(Path::new(&path)) {
    //         println!("{} ({} bytes)", path, md.len());
    //     }
    // }
    cache.start_watching()?;
    println!("Watching for changes — press Ctrl+C to stop.");

    // Ctrl+C stops watching gracefully: the watcher is released, the
    // archive flushes pending writes, and the process exits cleanly.
    unsafe {
        if libc::signal(libc::SIGINT, on_sigint as *const () as libc::sighandler_t) == libc::SIG_ERR
        {
            eprintln!("could not install SIGINT handler; killing the process instead");
        }
    }

    while !STOP.load(std::sync::atomic::Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    cache.stop_watching();
    println!("Stopped watching. Bye!");
    Ok(())
}
