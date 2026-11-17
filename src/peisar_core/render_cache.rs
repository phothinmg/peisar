#[derive(Default)]
pub(crate) struct RenderCache {
    enabled: bool,
    cached_html: Option<String>,
}
impl RenderCache {
    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.invalidate();
        }
    }

    pub(crate) fn get(&self) -> Option<String> {
        if self.enabled {
            self.cached_html.clone()
        } else {
            None
        }
    }

    pub(crate) fn store(&mut self, html: &str) {
        if self.enabled {
            self.cached_html = Some(html.to_string());
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.cached_html = None;
    }
}

#[cfg(test)]
mod tests {
    use super::RenderCache;

    #[test]
    fn cache_is_disabled_by_default_and_cleared_when_disabled() {
        let mut cache = RenderCache::default();
        cache.store("rendered");
        assert!(cache.get().is_none());

        cache.set_enabled(true);
        cache.store("rendered");
        assert_eq!(cache.get().as_deref(), Some("rendered"));

        cache.set_enabled(false);
        assert!(cache.get().is_none());
    }
}
