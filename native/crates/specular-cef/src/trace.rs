//! What the page hosts put in the resize ledger
//! ([`specular_core::ledger`]). Nothing here runs while the ledger is off.

use std::sync::{Mutex, PoisonError};

use specular_core::ledger::{self, Entry, PaintFacts};
use specular_core::{PageId, PixelSize};

use crate::page::PageGeometry;

/// The pages asked for a size that CEF has not read the view rect of since.
static UNREAD: Mutex<Vec<u64>> = Mutex::new(Vec::new());

/// A page host was asked for `geometry`, by `why`.
pub(crate) fn asked(page: PageId, geometry: &PageGeometry, why: &'static str) {
    if !ledger::enabled() {
        return;
    }
    ledger::ask(page.0, geometry.viewport, geometry.scale, why);
    let mut unread = UNREAD.lock().unwrap_or_else(PoisonError::into_inner);
    if !unread.contains(&page.0) {
        unread.push(page.0);
    }
}

/// CEF read the view rect of `page`: logged the first time after an ask.
pub(crate) fn view_rect_read(page: PageId, geometry: &PageGeometry) {
    if !ledger::enabled() {
        return;
    }
    let mut unread = UNREAD.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(at) = unread.iter().position(|&id| id == page.0) else {
        return;
    };
    unread.swap_remove(at);
    drop(unread);
    ledger::record(Entry::ViewRect(page.0, geometry.viewport));
}

/// CEF painted a shared texture of `coded` texels for `page`.
pub(crate) fn painted(
    page: PageId,
    coded: PixelSize,
    asked: PixelSize,
    extra: &cef::AcceleratedPaintInfoCommon,
) {
    if !ledger::enabled() {
        return;
    }
    let content = &extra.content_rect;
    ledger::record(Entry::Paint(PaintFacts {
        page: page.0,
        coded,
        matches: coded == asked,
        content: [content.x, content.y, content.width, content.height],
        source: (
            extra.source_size.width,
            extra.source_size.height,
            extra.has_source_size != 0,
        ),
        counter: (extra.capture_counter, extra.has_capture_counter != 0),
        timestamp: extra.timestamp,
    }));
}
