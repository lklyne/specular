//! A tab that is pressed and not yet clicked: its page is laid out for the
//! tab during the press, so a frame of the tab's size is there to draw when
//! the click shows it.
//!
//! Only the page's host follows the prepared item. What is shown, the
//! camera and `App::page_placement` do not, so the canvas goes on drawing
//! the page at the size it is stored at.

use specular_core::CssSize;
use specular_doc::EntityId;

use super::{Lens, fill_rect};
use crate::app::page_of;
use crate::{App, PagePlacement};

impl App {
    /// The page whose tab is pressed and whose host is already laid out as
    /// the tab will show it: at 100%, in view and painting. `None` once the
    /// tab is shown or the press is let go elsewhere.
    pub fn prepared_page(&self) -> Option<&EntityId> {
        self.session.prepared.as_ref()
    }
}

/// Lays out the host of `item` as its tab will, or with `None` lets go of
/// the page prepared. Only a page whose tab fills the view is laid out at
/// another size there, so anything else, and the tab already showing, is
/// left alone.
pub(crate) fn prepare(app: &mut App, item: Option<EntityId>) {
    app.session.prepared = item.filter(|item| {
        let fills = app.session.tabs.lens(item) == Lens::Fill;
        let page = app.document.entity(item).and_then(page_of).is_some();
        fills && page && app.shown_item() != Some(item) && app.session.gesture.is_none()
    });
}

/// The page prepared and the viewport its tab will lay it out at.
pub(crate) fn prepared(app: &App) -> Option<(EntityId, CssSize)> {
    let entity = app.document.entity(app.prepared_page()?)?;
    page_of(entity)?;
    let rect = fill_rect(app, entity)?;
    Some((entity.id.clone(), PagePlacement::viewport_for(rect)))
}

/// Forgets a prepared page that is now shown or gone.
pub(super) fn settle(app: &mut App) {
    let stands = app
        .prepared_page()
        .is_some_and(|item| app.shown_item() != Some(item) && app.document.entity(item).is_some());
    if !stands {
        app.session.prepared = None;
    }
}
