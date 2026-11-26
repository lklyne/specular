//! The per-page cap on shared textures held by the compositor.
//!
//! Each retained IOSurface keeps a Chromium capture-pool slot busy, so the
//! pool grows (allocating new surfaces) for as long as the compositor holds
//! frames. [`OutstandingFrames`] bounds that at
//! [`MAX_OUTSTANDING_TEXTURES`], like Electron's ADR 0038 pool cap: past it,
//! new paints are dropped instead of retained, so a slow compositor sheds
//! frames rather than ballooning GPU memory.
//!
//! A paint dropped at the cap leaves the page showing an older frame, so
//! the cap remembers it and the page is asked to paint again once a slot
//! is free ([`OutstandingFrames::take_repaint`]).
//!
//! Atomic because CEF may release handler objects (which hold a clone) on
//! any of its threads.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use specular_core::MAX_OUTSTANDING_TEXTURES;

/// Counts a page's live shared textures; cheap to clone (shared counter).
#[derive(Debug, Clone)]
pub(crate) struct OutstandingFrames {
    live: Arc<AtomicUsize>,
    /// Set when a paint was dropped at the cap.
    starved: Arc<AtomicBool>,
    cap: usize,
}

impl Default for OutstandingFrames {
    fn default() -> Self {
        Self::with_cap(MAX_OUTSTANDING_TEXTURES)
    }
}

impl OutstandingFrames {
    /// A counter with the given cap (tests use small caps).
    pub(crate) fn with_cap(cap: usize) -> Self {
        Self {
            live: Arc::new(AtomicUsize::new(0)),
            starved: Arc::new(AtomicBool::new(false)),
            cap,
        }
    }

    /// Takes a slot, or `None` when the cap is reached and the paint should
    /// be dropped. The slot frees when the lease drops.
    pub(crate) fn try_lease(&self) -> Option<FrameLease> {
        self.live
            .try_update(Ordering::AcqRel, Ordering::Acquire, |live| {
                (live < self.cap).then_some(live + 1)
            })
            .ok()?;
        Some(FrameLease {
            live: Arc::clone(&self.live),
        })
    }

    /// A paint was dropped at the cap: the page owes another.
    pub(crate) fn dropped(&self) {
        self.starved.store(true, Ordering::Release);
    }

    /// Whether a paint was dropped at the cap and a slot is free again, so
    /// the page should paint once more. Clears what it reports.
    pub(crate) fn take_repaint(&self) -> bool {
        self.live.load(Ordering::Acquire) < self.cap && self.starved.swap(false, Ordering::AcqRel)
    }
}

/// One held texture slot; moved into the `SharedTexture` release closure.
#[derive(Debug)]
pub(crate) struct FrameLease {
    live: Arc<AtomicUsize>,
}

impl Drop for FrameLease {
    fn drop(&mut self) {
        // Every lease was counted in by `try_lease`, so this never wraps.
        self.live.fetch_sub(1, Ordering::AcqRel);
    }
}
