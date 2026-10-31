//! One synthetic page: its spec, its scroll and the frames it paints.

use web_time::Instant;

use glam::Vec2;

use super::grid::synthetic_place;
use crate::frame::CpuFrame;
use crate::page::{PageId, PageSpec};
use crate::source::PageEvent;

#[derive(Debug)]
pub(super) struct SyntheticPage {
    pub(super) spec: PageSpec,
    pub(super) painting: bool,
    pub(super) frames_painted: u64,
    pub(super) next_paint: Option<Instant>,
    /// Every address the page has shown, oldest first.
    pub(super) history: Vec<String>,
    /// Which entry of `history` it shows now.
    pub(super) at: usize,
    /// How far its document is scrolled, in CSS pixels.
    pub(super) scroll: Vec2,
    /// Whether a devtools client enabled the `Page` domain, so navigations
    /// are reported to it.
    pub(super) page_events: bool,
    /// The selectors whose place the page reports.
    pub(super) tracked: Vec<String>,
}

impl SyntheticPage {
    /// The furthest the document scrolls: it is three viewports tall and
    /// one wide.
    pub(super) fn max_scroll(&self) -> Vec2 {
        Vec2::new(0.0, 2.0 * self.spec.viewport.height as f32)
    }

    /// Where every tracked selector's element sits, as a report of them all.
    pub(super) fn places(&self, id: PageId) -> Option<PageEvent> {
        (!self.tracked.is_empty()).then(|| PageEvent::ElementPlaces {
            page: id,
            places: (self.tracked.iter())
                .map(|selector| (selector.clone(), synthetic_place(selector)))
                .collect(),
        })
    }
}

/// The title a synthetic page gives the document at `url`: the address
/// without its scheme.
pub(super) fn title_of(url: &str) -> String {
    let address = url.split_once("://").map_or(url, |(_, rest)| rest);
    format!("Synthetic {}", address.trim_end_matches('/'))
}

/// Paints a flat colour that cycles with `frame_index` plus a sweeping bar,
/// so every frame differs and a stalled page is visible at a glance.
pub(super) fn paint(spec: &PageSpec, frame_index: u64) -> CpuFrame {
    let size = spec.viewport.to_pixels(spec.texture_scale);
    let stride = size.width * 4;
    let shade = (frame_index % 256) as u8;
    let bar_x = (frame_index * 8) % u64::from(size.width.max(1));
    let mut bgra = vec![0_u8; stride as usize * size.height as usize];
    for row in bgra.chunks_exact_mut(stride as usize) {
        let (texels, _) = row.as_chunks_mut::<4>();
        for (x, texel) in texels.iter_mut().enumerate() {
            let in_bar = (x as u64).abs_diff(bar_x) < 16;
            let value = if in_bar { 255 } else { shade };
            *texel = [value, 96, 255 - value, 255];
        }
    }
    CpuFrame {
        size,
        stride,
        bgra,
        dirty: Vec::new(),
    }
}
