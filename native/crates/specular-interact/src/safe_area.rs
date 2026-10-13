//! A page in a device frame and the safe area of that device's screen.
//!
//! A phone lays a document out inside the safe area of its screen and
//! paints the strips left over in the document's colour, unless the
//! document asks for the whole screen with `viewport-fit=cover`. Then it
//! gets the screen and is told the insets, as `env(safe-area-inset-*)`.
//!
//! The entity's rect stays the device's screen. What changes is where the
//! document sits in it: [`App::page_placement`] is the one place that says,
//! so every point mapped into a page goes through the same inset.

use specular_core::{CssSize, PageDevice, PageSafeArea};
use specular_doc::{EntityId, Page, Rect, SafeArea};

use crate::app::page_of;
use crate::{App, Effect, ScreenFit};

/// The colour round a document that names none: a browser's own white.
const BLANK: [u8; 3] = [255, 255, 255];

impl App {
    /// The safe area of the screen the page `id` is shown as: its device's
    /// while its frame is drawn. None where the page fills the view, since
    /// it is a browser tab there and no device.
    pub fn page_safe_area(&self, id: &EntityId) -> SafeArea {
        if crate::showing::fills(self, id) {
            return SafeArea::NONE;
        }
        (self.document.entity(id).and_then(page_of)).map_or(SafeArea::NONE, Page::safe_area)
    }

    /// How far in from each edge of its screen the document of the page
    /// `id` is laid out: the safe area, or nothing for a document that
    /// covers the screen.
    pub fn page_inset(&self, id: &EntityId) -> SafeArea {
        if self.covers(id) {
            return SafeArea::NONE;
        }
        self.page_safe_area(id)
    }

    /// Whether the document of the page `id` has said it covers its screen.
    fn covers(&self, id: &EntityId) -> bool {
        self.page_state(id)
            .is_some_and(|state| state.fit == ScreenFit::Covers)
    }

    /// The colour of the screen round the document of the page `id`, as
    /// red, green and blue: the document's, or white until it names one.
    pub fn page_surround(&self, id: &EntityId) -> [u8; 3] {
        (self.page_state(id))
            .and_then(|state| state.surround)
            .unwrap_or(BLANK)
    }

    /// The `env(safe-area-inset-*)` the page `id` is told: the safe area
    /// for a document that covers the screen, and none for one laid out
    /// inside it.
    pub(crate) fn page_env(&self, id: &EntityId) -> PageSafeArea {
        if !self.covers(id) {
            return PageSafeArea::default();
        }
        let area = self.page_safe_area(id);
        let whole = |inset: f64| inset.round().max(0.0) as u32;
        PageSafeArea {
            top: whole(area.top),
            right: whole(area.right),
            bottom: whole(area.bottom),
            left: whole(area.left),
        }
    }
}

/// The part of `screen` inside `area`. The whole of a screen too small to
/// have anything left inside.
pub fn within(screen: Rect, area: SafeArea) -> Rect {
    let width = screen.width - area.left - area.right;
    let height = screen.height - area.top - area.bottom;
    if width < 1.0 || height < 1.0 {
        return screen;
    }
    Rect::new(screen.x + area.left, screen.y + area.top, width, height)
}

/// How a page is laid out and what it is told of its screen.
pub(crate) type Fitted = Option<(CssSize, PageDevice)>;

/// The page `id` as its host has it, for [`follow_fit`] to compare with
/// once the page has said how it fits its screen.
pub(crate) fn fitted(app: &App, id: &EntityId) -> Fitted {
    Some((app.page_placement(id)?.viewport, app.page_device(id)))
}

/// Appends the changes that take the host of the page `id` from `before`
/// to how the page fits its screen now.
pub(crate) fn follow_fit(before: Fitted, app: &App, id: &EntityId, effects: &mut Vec<Effect>) {
    let (Some((viewport, device)), Some((now, device_now))) = (before, fitted(app, id)) else {
        return;
    };
    if device != device_now {
        effects.push(Effect::SetPageDevice {
            page: id.clone(),
            device: device_now,
        });
    }
    if viewport != now {
        effects.push(Effect::SetPageViewport {
            page: id.clone(),
            viewport: now,
        });
    }
}
