//! The textures the compositor holds for each page.

use std::time::{Duration, Instant};

use specular_core::{CssSize, PixelRect, PixelSize, SharedTexture};

use crate::draw_list::{LayerKind, PageLayersInfo};

/// The current texture of one page layer.
#[derive(Debug)]
pub(crate) struct LayerTexture {
    pub(crate) texture: wgpu::Texture,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) size: PixelSize,
    /// The producer's surface backing `texture`, kept checked out while it is
    /// displayed; `None` for CPU uploads.
    pub(crate) shared: Option<SharedTexture>,
    pub(crate) produced_at: Instant,
    /// Whether this frame has been drawn yet (for paint-to-submit latency).
    pub(crate) shown: bool,
}

/// A popup widget's state (CEF `OnPopupShow` / `OnPopupSize` / `PET_POPUP`).
#[derive(Debug, Default)]
pub(crate) struct PopupLayer {
    pub(crate) visible: bool,
    pub(crate) rect: Option<PixelRect>,
    pub(crate) texture: Option<LayerTexture>,
}

/// How long a kept frame that is not being drawn is held: long enough to go
/// back to the size it was painted at without a bare frame, and no longer,
/// since it pins a surface of its page.
pub(crate) const KEPT_FOR: Duration = Duration::from_secs(2);

/// Everything held for one page.
///
/// A page whose host is resized paints at the new size some frames later,
/// and it may be resized before the scene lays it out at that size, so two
/// view frames are held: the newest, and the newest of one other CSS size.
/// A draw takes the one painted at the size the scene asks for.
#[derive(Debug, Default)]
pub(crate) struct PageLayers {
    pub(crate) view: Option<LayerTexture>,
    /// The CSS size of the view the view texture shows.
    pub(crate) view_css: CssSize,
    /// The newest view frame of another CSS size than `view`'s.
    pub(crate) kept: Option<LayerTexture>,
    /// The CSS size of the view the kept texture shows.
    pub(crate) kept_css: CssSize,
    /// When the kept frame was set aside.
    kept_at: Option<Instant>,
    /// Whether the latest draw of this page took the kept frame.
    kept_drawn: bool,
    pub(crate) popup: PopupLayer,
}

/// Which held view frame a rect laid out at `wanted` CSS pixels is drawn
/// from, given the size of the newest and of the kept one: the one painted
/// at that size, else the newest.
pub(crate) fn view_for(wanted: CssSize, newest: CssSize, kept: Option<CssSize>) -> LayerKind {
    if wanted != newest && kept == Some(wanted) {
        LayerKind::KeptView
    } else {
        LayerKind::View
    }
}

impl PageLayers {
    /// The layer slot for `kind`.
    pub(crate) fn slot_mut(&mut self, kind: LayerKind) -> &mut Option<LayerTexture> {
        match kind {
            LayerKind::View => &mut self.view,
            LayerKind::KeptView => &mut self.kept,
            LayerKind::Popup => &mut self.popup.texture,
        }
    }

    /// The texture for `kind`, if painted.
    pub(crate) fn get(&self, kind: LayerKind) -> Option<&LayerTexture> {
        match kind {
            LayerKind::View => self.view.as_ref(),
            LayerKind::KeptView => self.kept.as_ref(),
            LayerKind::Popup => self.popup.texture.as_ref(),
        }
    }

    /// Makes room for a view frame painted at `css`. A frame of the size
    /// held is about to be replaced and stays where it is. One of another
    /// size is set aside at `now`, and the frame that was kept until now is
    /// returned to be retired.
    pub(crate) fn make_room(&mut self, css: CssSize, now: Instant) -> Option<LayerTexture> {
        if self.view.is_none() || self.view_css == css {
            self.view_css = css;
            return None;
        }
        let evicted = std::mem::replace(&mut self.kept, self.view.take());
        self.kept_css = std::mem::replace(&mut self.view_css, css);
        self.kept_at = Some(now);
        self.kept_drawn = false;
        evicted
    }

    /// Notes which view frame the latest draw of this page took.
    pub(crate) fn drew(&mut self, kind: LayerKind) {
        match kind {
            LayerKind::View => self.kept_drawn = false,
            LayerKind::KeptView => self.kept_drawn = true,
            LayerKind::Popup => {}
        }
    }

    /// Takes the kept frame once it has been held for [`KEPT_FOR`] and the
    /// latest draw did not use it.
    pub(crate) fn take_idle_kept(&mut self, now: Instant) -> Option<LayerTexture> {
        let old = (self.kept_at).is_some_and(|at| now.saturating_duration_since(at) >= KEPT_FOR);
        if self.kept_drawn || !old {
            return None;
        }
        self.kept_at = None;
        self.kept.take()
    }

    /// Shared surfaces currently displayed (not counting retired ones).
    pub(crate) fn shared_count(&self) -> usize {
        [
            self.view.as_ref(),
            self.kept.as_ref(),
            self.popup.texture.as_ref(),
        ]
        .into_iter()
        .flatten()
        .filter(|layer| layer.shared.is_some())
        .count()
    }

    /// What the draw list needs to know to draw the page in a rect laid out
    /// at `wanted` CSS pixels; `None` until the view has painted.
    pub(crate) fn info(&self, wanted: CssSize) -> Option<PageLayersInfo> {
        let newest = (self.view.as_ref()).map(|view| (LayerKind::View, view, self.view_css));
        let kept = (self.kept.as_ref()).map(|kept| (LayerKind::KeptView, kept, self.kept_css));
        // With no newest frame, as after one that could not be imported,
        // the kept one is all there is.
        let takes_kept = match (&newest, &kept) {
            (Some(newest), Some(kept)) => view_for(wanted, newest.2, Some(kept.2)) == kept.0,
            (None, Some(_)) => true,
            (Some(_) | None, None) => false,
        };
        let (layer, view, view_css) = if takes_kept { kept? } else { newest? };
        // A popup is placed in the newest frame's texels.
        let popup = match (&self.popup, self.popup.texture.is_some(), layer) {
            (
                PopupLayer {
                    visible: true,
                    rect: Some(rect),
                    ..
                },
                true,
                LayerKind::View,
            ) => Some(*rect),
            _ => None,
        };
        Some(PageLayersInfo {
            view: layer,
            view_size: view.size,
            view_css,
            view_is_cpu: view.shared.is_none(),
            popup,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Breaks when a draw takes the newest frame whatever its size (the
    /// flash of a tab's frame in the canvas's rect) or prefers the kept one
    /// when neither fits.
    #[test]
    fn a_draw_takes_the_frame_painted_at_its_size_else_the_newest() {
        let (canvas, tab, other) = (
            CssSize::new(400, 300),
            CssSize::new(1400, 800),
            CssSize::new(900, 600),
        );
        let rows = [
            (canvas, tab, Some(canvas), LayerKind::KeptView),
            (tab, tab, Some(canvas), LayerKind::View),
            (other, tab, Some(canvas), LayerKind::View),
            (canvas, tab, None, LayerKind::View),
            (tab, tab, Some(tab), LayerKind::View),
        ];
        for (wanted, newest, kept, layer) in rows {
            assert_eq!(view_for(wanted, newest, kept), layer, "{wanted:?} {kept:?}");
        }
    }
}
