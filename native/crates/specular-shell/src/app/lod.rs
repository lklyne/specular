//! The Electron page-host LOD: each page's texture scale, frame rate and
//! painting follow how large it is on screen and whether it is in view.

use std::time::Instant;

use glam::Vec2;
use specular_bench::PaintPolicy;
use specular_core::{PageId, PageSource};
use specular_interact::to_canvas_rect;

use super::runtime::{Runtime, ShellWindow};
use crate::paint_lod::{FULL_FRAME_RATE, LodChange};

impl<W: ShellWindow> Runtime<W> {
    /// The window moved to a display of another scale: every page is asked
    /// for frames at it.
    pub fn on_scale_factor_changed(&mut self, scale_factor: f64) {
        self.demand.changed();
        for host in self.hosts.values() {
            let scale = scale_factor as f32 * host.lod.texture().factor();
            if let Err(error) = self.source.set_texture_scale(host.page, scale) {
                tracing::warn!("{error}");
            }
        }
    }

    /// Grades the pages, then sends each one whose viewport or scale changed
    /// this turn its new geometry as one resize: a page shown at a new size
    /// and a new scale is not first painted at the old scale.
    pub(super) fn settle_pages(&mut self, now: Instant) {
        self.pages_settled = true;
        if self.options.paint_policy == PaintPolicy::ElectronLod
            && let Some(viewport) = self.gpu.as_ref().map(W::logical_viewport)
        {
            self.update_paint_lod(viewport, now);
        }
        self.source.flush_geometry();
    }

    /// One layout pass: grades every page by its on-screen scale and
    /// visibility and applies what changed. The page the user is in, entered
    /// or filling its tab, paints at the display's rate. In a window no one
    /// can see, no page paints.
    fn update_paint_lod(&mut self, viewport: Vec2, now: Instant) {
        let Some(window) = self.gpu.as_ref() else {
            return;
        };
        let window_scale = window.scale_factor();
        let display_rate = window.refresh_rate().unwrap_or(FULL_FRAME_RATE);
        let seen = !window.occluded();
        let camera = self.app.session().camera;
        let prepared = self.app.prepared_page();
        let entered = self.app.session().focus.page();
        let filling = self.app.filling_item();
        for (id, placement) in self.app.pages() {
            let Some(host) = self.hosts.get_mut(id) else {
                continue;
            };
            // A page prepared for its tab is graded as the tab will show it,
            // so its host gets the tab's viewport and scale as one resize.
            let (display_scale, on_screen) = if prepared == Some(id) {
                (1.0, true)
            } else {
                (
                    placement.display_scale(&camera),
                    camera.is_visible(to_canvas_rect(placement.rect), viewport),
                )
            };
            let in_use = [prepared, entered, filling].contains(&Some(id));
            let focus_rate = in_use.then_some(display_rate);
            let change = host
                .lod
                .update(display_scale, on_screen && seen, focus_rate, now);
            apply_lod_change(self.source.as_mut(), host.page, change, window_scale);
        }
    }
}

/// Applies `change` in the order Electron's layout pass does: scale before
/// painting, so a page coming into view wakes at the scale it is owed.
fn apply_lod_change(
    source: &mut dyn PageSource,
    page: PageId,
    change: LodChange,
    window_scale: f32,
) {
    if change != LodChange::default() {
        tracing::debug!(%page, ?change, "paint LOD");
    }
    let results = [
        change
            .texture
            .map(|tier| source.set_texture_scale(page, window_scale * tier.factor())),
        change
            .frame_rate
            .map(|fps| source.set_frame_rate(page, fps)),
        change
            .painting
            .map(|painting| source.set_painting(page, painting)),
    ];
    for error in results.into_iter().flatten().filter_map(Result::err) {
        tracing::warn!("{error}");
    }
}
