//! The textures the compositor holds for each page.

use std::time::Instant;

use specular_core::{PixelRect, PixelSize, SharedTexture};

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

/// Everything held for one page.
#[derive(Debug, Default)]
pub(crate) struct PageLayers {
    pub(crate) view: Option<LayerTexture>,
    pub(crate) popup: PopupLayer,
}

impl PageLayers {
    /// The layer slot for `kind`.
    pub(crate) fn slot_mut(&mut self, kind: LayerKind) -> &mut Option<LayerTexture> {
        match kind {
            LayerKind::View => &mut self.view,
            LayerKind::Popup => &mut self.popup.texture,
        }
    }

    /// The texture for `kind`, if painted.
    pub(crate) fn get(&self, kind: LayerKind) -> Option<&LayerTexture> {
        match kind {
            LayerKind::View => self.view.as_ref(),
            LayerKind::Popup => self.popup.texture.as_ref(),
        }
    }

    /// Shared surfaces currently displayed (not counting retired ones).
    pub(crate) fn shared_count(&self) -> usize {
        [self.view.as_ref(), self.popup.texture.as_ref()]
            .into_iter()
            .flatten()
            .filter(|layer| layer.shared.is_some())
            .count()
    }

    /// What the draw list needs to know; `None` until the view has painted.
    pub(crate) fn info(&self) -> Option<PageLayersInfo> {
        let view = self.view.as_ref()?;
        let popup = match (&self.popup, self.popup.texture.is_some()) {
            (
                PopupLayer {
                    visible: true,
                    rect: Some(rect),
                    ..
                },
                true,
            ) => Some(*rect),
            _ => None,
        };
        Some(PageLayersInfo {
            view_size: view.size,
            view_is_cpu: view.shared.is_none(),
            popup,
        })
    }
}
