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
//! frame size changes, and with the page. A layer that has stopped painting
//! counts no ingests, so after [`IDLE_TIME`] without one it keeps only the
//! surface it last showed: a still page pins one surface, not a pool.

use web_time::{Duration, Instant};

/// Most surfaces remembered per layer; above Chromium's capture-pool depth,
/// so a steady pool never misses.
pub(crate) const CAPACITY: usize = 8;

/// Ingests an entry may go unused before it is dropped (about 2 s of
/// 60 fps paints), so surfaces Chromium has retired stop pinning memory.
pub(crate) const IDLE_INGESTS: u64 = 120;

/// How long a layer may go without a paint before its cache lets go of
/// every surface but the latest.
pub(crate) const IDLE_TIME: Duration = Duration::from_secs(2);

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
    /// When the layer last painted.
    painted_at: Option<Instant>,
}

impl<K, V> Default for ImportCache<K, V> {
    fn default() -> Self {
        Self {
            entries: Vec::with_capacity(CAPACITY),
            clock: 0,
            hits: 0,
            misses: 0,
            painted_at: None,
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

    /// Notes that the layer painted at `now`.
    pub(crate) fn painted(&mut self, now: Instant) {
        self.painted_at = Some(now);
    }

    /// Drops every surface but the most recently used once the layer has
    /// not painted for [`IDLE_TIME`]. Returns how many were dropped.
    pub(crate) fn settle(&mut self, now: Instant) -> usize {
        let idle =
            (self.painted_at).is_some_and(|at| now.saturating_duration_since(at) >= IDLE_TIME);
        if !idle || self.entries.len() <= 1 {
            return 0;
        }
        let latest = self.entries.iter().map(|entry| entry.last_used).max();
        let before = self.entries.len();
        self.entries.retain(|entry| Some(entry.last_used) == latest);
        before - self.entries.len()
    }

    /// Drops every entry whose key fails `keep` (e.g. a stale frame size).
    pub(crate) fn retain(&mut self, keep: impl Fn(&K) -> bool) {
        self.entries.retain(|entry| keep(&entry.key));
    }

    /// Lookups served from the cache, and imports made, so far.
    pub(crate) fn hits_and_misses(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;

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
}
