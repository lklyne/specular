//! Pages: live web items on the canvas (JSON Canvas `link` nodes on disk).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::geometry::{CanvasRect, CssSize};

/// Identity of a page within one running [`PageSource`](crate::PageSource).
///
/// Allocated by the source on [`create_page`](crate::PageSource::create_page);
/// it is a process-local handle, not the persisted `.canvas` node id (which
/// lives on [`Page::node_id`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PageId(pub u64);

impl fmt::Display for PageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "page#{}", self.0)
    }
}

/// What a [`PageSource`](crate::PageSource) needs to host a page.
#[derive(Debug, Clone, PartialEq)]
pub struct PageSpec {
    /// Full URL including scheme and host.
    pub url: String,
    /// Layout viewport in CSS pixels (the page's breakpoint).
    pub viewport: CssSize,
    /// Texels per CSS pixel to raster at — the page's *texture scale*
    /// (CONTEXT.md, "Page textures"). Maps to CEF's device scale factor.
    pub texture_scale: f32,
    /// Target paint rate in frames per second (the page's *frame-rate LOD*).
    pub frame_rate: u32,
}

impl PageSpec {
    /// A spec at texture scale 1 and 60 fps.
    pub fn new(url: &str, viewport: CssSize) -> Self {
        Self {
            url: url.to_owned(),
            viewport,
            texture_scale: 1.0,
            frame_rate: 60,
        }
    }
}

/// A page placed on the canvas: where it sits and what it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// Runtime handle from the page source.
    pub id: PageId,
    /// The `.canvas` node id this page was loaded from (stable across runs).
    pub node_id: String,
    /// Full URL.
    pub url: String,
    /// Placement in canvas space. Its size is the CSS viewport at zoom 1.
    pub rect: CanvasRect,
    /// Layout viewport in CSS pixels.
    pub viewport: CssSize,
}
