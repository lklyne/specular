//! Imported shared surfaces, kept per page layer so a recycled surface is
//! not re-imported.
//!
//! Chromium paints each page into a small rotating pool of `IOSurface`s, so
//! almost every `OnAcceleratedPaint` repeats a surface seen a few frames
//! earlier. Importing it again would create a Metal texture, a wgpu texture,
//! a view and a bind group per paint on the very path being measured; the
//! cache keeps those per surface instead.
//!
//! A cached texture's `MTLTexture` retains its IOSurface, so while an entry
//! lives no other surface can take its ID and the key cannot alias.
//! Entries go when they have been idle for [`IDLE_INGESTS`] ingests, when the
//! cache is full ([`CAPACITY`], least recently used first), when the layer's
//! frame size changes, and with the page.

/// Most surfaces remembered per layer; above Chromium's capture-pool depth,
/// so a steady pool never misses.
pub(crate) const CAPACITY: usize = 8;

/// Ingests an entry may go unused before it is dropped (about 2 s of
/// 60 fps paints), so surfaces Chromium has retired stop pinning memory.
pub(crate) const IDLE_INGESTS: u64 = 120;

#[derive(Debug)]
struct Entry<K, V> {
    key: K,
    value: V,
    last_used: u64,
}

/// A small LRU of imports for one page layer, keyed by surface identity.
#[derive(Debug)]
pub(crate) struct ImportCache<K, V> {
    entries: Vec<Entry<K, V>>,
    clock: u64,
    hits: u64,
    misses: u64,
}

impl<K, V> Default for ImportCache<K, V> {
    fn default() -> Self {
        Self {
            entries: Vec::with_capacity(CAPACITY),
            clock: 0,
            hits: 0,
            misses: 0,
        }
    }
}

impl<K: PartialEq, V> ImportCache<K, V> {
    /// The cached value for `key`, or the one `import` makes (then cached).
    /// A failed import caches nothing.
    pub(crate) fn get_or_import<E>(
        &mut self,
        key: K,
        import: impl FnOnce() -> Result<V, E>,
    ) -> Result<&V, E> {
        self.clock += 1;
        let clock = self.clock;
        self.entries
            .retain(|entry| clock.saturating_sub(entry.last_used) <= IDLE_INGESTS);
        if let Some(index) = self.entries.iter().position(|entry| entry.key == key) {
            self.hits += 1;
            let entry = &mut self.entries[index];
            entry.last_used = clock;
            return Ok(&entry.value);
        }
        let value = import()?;
        self.misses += 1;
        if self.entries.len() >= CAPACITY
            && let Some(oldest) = self
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(index, _)| index)
        {
            self.entries.swap_remove(oldest);
        }
        self.entries.push(Entry {
            key,
            value,
            last_used: clock,
        });
        Ok(&self.entries[self.entries.len() - 1].value)
    }

    /// Drops every entry whose key fails `keep` (e.g. a stale frame size).
    pub(crate) fn retain(&mut self, keep: impl Fn(&K) -> bool) {
        self.entries.retain(|entry| keep(&entry.key));
    }

    /// Lookups served from the cache, and imports made, so far.
    pub(crate) fn hits_and_misses(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;

    fn ok(value: u32) -> impl FnOnce() -> Result<u32, Infallible> {
        move || Ok(value)
    }

    #[test]
    fn repeated_surface_is_imported_once() {
        let mut cache = ImportCache::default();
        let mut imports = 0;
        for _ in 0..3 {
            let _ = cache.get_or_import::<Infallible>(1, || {
                imports += 1;
                Ok(imports)
            });
        }
        assert_eq!(imports, 1);
    }

    #[test]
    fn hits_and_misses_count_lookups() {
        let mut cache = ImportCache::default();
        for key in [1, 2, 1, 1] {
            let _ = cache.get_or_import(key, ok(key));
        }
        assert_eq!(cache.hits_and_misses(), (2, 2));
    }

    #[test]
    fn failed_import_is_not_cached() {
        let mut cache: ImportCache<u32, u32> = ImportCache::default();
        let _ = cache.get_or_import(1, || Err("refused"));
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn full_cache_evicts_least_recently_used() {
        let mut cache = ImportCache::default();
        for key in 0..CAPACITY as u32 {
            let _ = cache.get_or_import(key, ok(key));
        }
        let _ = cache.get_or_import(0, ok(0));
        let _ = cache.get_or_import(99, ok(99));
        let mut reimported = false;
        let _ = cache.get_or_import::<Infallible>(1, || {
            reimported = true;
            Ok(1)
        });
        assert!(reimported);
    }

    #[test]
    fn idle_entries_are_dropped() {
        let mut cache = ImportCache::default();
        let _ = cache.get_or_import(1, ok(1));
        for _ in 0..=IDLE_INGESTS {
            let _ = cache.get_or_import(2, ok(2));
        }
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn retain_drops_rejected_keys() {
        let mut cache = ImportCache::default();
        let _ = cache.get_or_import(1, ok(1));
        let _ = cache.get_or_import(2, ok(2));
        cache.retain(|key| *key == 2);
        assert_eq!(cache.len(), 1);
    }
}
