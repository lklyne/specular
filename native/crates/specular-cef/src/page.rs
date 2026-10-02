//! Per-page state shared between a page's CEF handlers and the page source.
//!
//! [`PageGeometry`] is pure and compiled everywhere. With the `cef` feature,
//! `PageContext` bundles it with the texture cap and the browser count.
//!
//! CEF invokes the handlers on the browser UI thread, which in this
//! configuration (`multi_threaded_message_loop = 0`, pumped from the winit
//! loop) is the process main thread. CEF still reference-counts handler
//! objects from other threads (the IO thread fetches the request handler for
//! every network request), so whichever thread drops the last reference runs
//! the handler's destructor. Everything a handler holds is therefore
//! `Send + Sync` (`Arc`, `Mutex`, atomics). Events, which carry `!Send`
//! shared textures, never live in a handler: they go into a queue local to
//! the UI thread, and a callback arriving on any other thread is logged and
//! its event dropped rather than raced.

#[cfg(feature = "cef")]
use std::cell::RefCell;
#[cfg(feature = "cef")]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(feature = "cef")]
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
#[cfg(feature = "cef")]
use std::thread::{self, ThreadId};

use specular_core::{CssSize, PageSpec, PixelRect, PixelSize};
#[cfg(feature = "cef")]
use specular_core::{PageEvent, PageId};

use crate::coords::place_popup;
#[cfg(feature = "cef")]
use crate::pool::OutstandingFrames;

/// Geometry the render handler reports to CEF and uses to place popups.
#[derive(Debug, Clone, PartialEq)]
pub struct PageGeometry {
    /// CSS viewport (`GetViewRect`).
    pub viewport: CssSize,
    /// Device scale factor (`GetScreenInfo`).
    pub scale: f32,
    /// Last `OnPopupSize` rect in CSS, kept to re-place on scale changes.
    popup_css: Option<PixelRect>,
}

impl PageGeometry {
    /// Geometry for a newly created page.
    pub fn new(spec: &PageSpec) -> Self {
        Self {
            viewport: spec.viewport,
            scale: spec.texture_scale,
            popup_css: None,
        }
    }

    /// View frame size in texels.
    pub fn view_texels(&self) -> PixelSize {
        self.viewport.to_pixels(self.scale)
    }

    /// Records a popup move/resize and returns its texel placement.
    pub fn set_popup(&mut self, css: PixelRect) -> PixelRect {
        self.popup_css = Some(css);
        place_popup(css, self.scale, self.view_texels())
    }

    /// Forgets the popup when it closes.
    pub fn clear_popup(&mut self) {
        self.popup_css = None;
    }

    /// Current popup placement in texels, if a popup is open.
    pub fn popup(&self) -> Option<PixelRect> {
        self.popup_css
            .map(|css| place_popup(css, self.scale, self.view_texels()))
    }
}

#[cfg(feature = "cef")]
thread_local! {
    /// Events raised by handlers on the UI thread, drained by the source.
    static EVENTS: RefCell<Vec<PageEvent>> = const { RefCell::new(Vec::new()) };
}

/// Moves every queued event into `out`. Call on the UI thread.
#[cfg(feature = "cef")]
pub(crate) fn drain_events(out: &mut Vec<PageEvent>) {
    EVENTS.with_borrow_mut(|events| out.append(events));
}

/// Drops every queued event (releasing any shared textures they hold).
#[cfg(feature = "cef")]
pub(crate) fn clear_events() {
    EVENTS.with_borrow_mut(Vec::clear);
}

/// Everything a page's handlers need; cheap to clone, `Send + Sync`.
#[cfg(feature = "cef")]
#[derive(Debug, Clone)]
pub(crate) struct PageContext {
    /// The page these handlers belong to.
    pub id: PageId,
    /// Geometry, mutated by the source on resize/rescale.
    pub geometry: Arc<Mutex<PageGeometry>>,
    /// The page's shared-texture cap.
    #[cfg_attr(
        not(target_os = "macos"),
        expect(dead_code, reason = "shared textures are only taken on macOS")
    )]
    pub frames: OutstandingFrames,
    /// Browsers created and not yet through `OnBeforeClose`, for shutdown.
    pub alive: Arc<AtomicUsize>,
    /// The thread that owns the event queue (the CEF UI thread).
    pub ui_thread: ThreadId,
}

#[cfg(feature = "cef")]
impl PageContext {
    /// Locks the geometry. A panic while it was held cannot leave it
    /// half-updated (every write is one field), so poisoning is ignored.
    pub(crate) fn geometry(&self) -> MutexGuard<'_, PageGeometry> {
        lock_geometry(&self.geometry)
    }

    /// Queues `event` for the source, if called on the UI thread.
    pub(crate) fn push(&self, event: PageEvent) {
        if thread::current().id() == self.ui_thread {
            EVENTS.with_borrow_mut(|events| events.push(event));
        } else {
            tracing::warn!(page = %self.id, "CEF callback off the UI thread; event dropped");
        }
    }

    /// Records that a browser finished closing.
    pub(crate) fn browser_closed(&self) {
        // Saturating: a stray extra OnBeforeClose must not wrap the count.
        let _ = self
            .alive
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1));
    }
}

/// Handlers may be destroyed on any CEF thread; keep their state thread-safe.
#[cfg(feature = "cef")]
const _: fn() = || {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<PageContext>();
};

/// Locks `geometry`, ignoring poisoning (see [`PageContext::geometry`]).
#[cfg(feature = "cef")]
pub(crate) fn lock_geometry(geometry: &Mutex<PageGeometry>) -> MutexGuard<'_, PageGeometry> {
    geometry.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_is_re_placed_when_scale_changes() {
        let mut geometry = PageGeometry::new(&PageSpec::new("https://a/", CssSize::new(400, 300)));
        geometry.set_popup(PixelRect::new(10, 10, 50, 20));
        geometry.scale = 2.0;
        assert_eq!(geometry.popup(), Some(PixelRect::new(20, 20, 100, 40)));
    }

    #[test]
    fn closed_popup_has_no_placement() {
        let mut geometry = PageGeometry::new(&PageSpec::new("https://a/", CssSize::new(400, 300)));
        geometry.set_popup(PixelRect::new(10, 10, 50, 20));
        geometry.clear_popup();
        assert_eq!(geometry.popup(), None);
    }
}
