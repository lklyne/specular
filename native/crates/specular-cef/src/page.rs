//! Per-page state shared between a page's CEF handlers and the page source.
//!
//! [`PageGeometry`] is pure and compiled everywhere. With the `cef` feature,
//! `PageContext` bundles it with the event queue and texture cap.
//!
//! CEF calls the handlers on the browser UI thread, which in this
//! configuration (`multi_threaded_message_loop = 0`, pumped from the winit
//! loop) is the process main thread, the same thread every `PageSource`
//! method runs on. Plain `Rc<RefCell<_>>` is therefore sound. Callbacks only
//! fire inside `CefDoMessageLoopWork` or synchronously inside a host call,
//! so the source never holds a borrow across a CEF call.

#[cfg(feature = "cef")]
use std::cell::{Cell, RefCell};
#[cfg(feature = "cef")]
use std::rc::Rc;

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

/// Everything a page's handlers need; cheap to clone (all shared).
#[cfg(feature = "cef")]
#[derive(Debug, Clone)]
pub(crate) struct PageContext {
    /// The page these handlers belong to.
    pub id: PageId,
    /// Geometry, mutated by the source on resize/rescale.
    pub geometry: Rc<RefCell<PageGeometry>>,
    /// Source-wide event queue, drained by `PageSource::drain_events`.
    pub events: Rc<RefCell<Vec<PageEvent>>>,
    /// The page's shared-texture cap.
    #[cfg_attr(
        not(target_os = "macos"),
        expect(dead_code, reason = "shared textures are only taken on macOS")
    )]
    pub frames: OutstandingFrames,
    /// Browsers created and not yet through `OnBeforeClose`, for shutdown.
    pub alive: Rc<Cell<usize>>,
}

#[cfg(feature = "cef")]
impl PageContext {
    pub(crate) fn push(&self, event: PageEvent) {
        self.events.borrow_mut().push(event);
    }
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
