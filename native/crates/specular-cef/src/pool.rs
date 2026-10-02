//! The per-page cap on shared textures held by the compositor.
//!
//! Each retained IOSurface keeps a Chromium capture-pool slot busy, so the
//! pool grows (allocating new surfaces) for as long as the compositor holds
//! frames. [`OutstandingFrames`] bounds that at
//! [`MAX_OUTSTANDING_TEXTURES`], like Electron's ADR 0038 pool cap: past it,
//! new paints are dropped instead of retained, so a slow compositor sheds
//! frames rather than ballooning GPU memory.
//!
//! Atomic because CEF may release handler objects (which hold a clone) on
//! any of its threads.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use specular_core::MAX_OUTSTANDING_TEXTURES;

/// Counts a page's live shared textures; cheap to clone (shared counter).
#[derive(Debug, Clone)]
pub struct OutstandingFrames {
    live: Arc<AtomicUsize>,
    cap: usize,
}

impl Default for OutstandingFrames {
    fn default() -> Self {
        Self::with_cap(MAX_OUTSTANDING_TEXTURES)
    }
}

impl OutstandingFrames {
    /// A counter with the given cap (tests use small caps).
    pub fn with_cap(cap: usize) -> Self {
        Self {
            live: Arc::new(AtomicUsize::new(0)),
            cap,
        }
    }

    /// Takes a slot, or `None` when the cap is reached and the paint should
    /// be dropped. The slot frees when the lease drops.
    pub fn try_lease(&self) -> Option<FrameLease> {
        self.live
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |live| {
                (live < self.cap).then_some(live + 1)
            })
            .ok()?;
        Some(FrameLease {
            live: Arc::clone(&self.live),
        })
    }

    /// Textures currently held.
    pub fn live(&self) -> usize {
        self.live.load(Ordering::Acquire)
    }
}

/// One held texture slot; moved into the `SharedTexture` release closure.
#[derive(Debug)]
pub struct FrameLease {
    live: Arc<AtomicUsize>,
}

impl Drop for FrameLease {
    fn drop(&mut self) {
        // Every lease was counted in by `try_lease`, so this never wraps.
        self.live.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lease_is_refused_at_the_cap() {
        let pool = OutstandingFrames::with_cap(2);
        let _a = pool.try_lease();
        let _b = pool.try_lease();
        assert!(pool.try_lease().is_none());
    }

    #[test]
    fn dropping_a_lease_frees_its_slot() {
        let pool = OutstandingFrames::with_cap(1);
        let lease = pool.try_lease();
        drop(lease);
        assert!(pool.try_lease().is_some());
    }

    #[test]
    fn clones_share_one_counter() {
        let pool = OutstandingFrames::default();
        let clone = pool.clone();
        let _lease = clone.try_lease();
        assert_eq!(pool.live(), 1);
    }

    #[test]
    fn default_cap_matches_electron_pool_cap() {
        let pool = OutstandingFrames::default();
        let leases: Vec<_> = std::iter::from_fn(|| pool.try_lease()).take(100).collect();
        assert_eq!(leases.len(), MAX_OUTSTANDING_TEXTURES);
    }
}
