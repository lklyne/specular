//! What a page's quads sample, and where its popup goes.

use glam::Vec2;
use specular_core::{CssSize, PageId, PixelRect, PixelSize};

use crate::gpu_types::QuadInstance;

/// Which of a page's textures a quad samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LayerKind {
    /// The page's main view: its newest frame.
    View,
    /// The view frame kept from another size than the newest.
    KeptView,
    /// The popup widget drawn over it.
    Popup,
}

/// One draw call: quad `instance` of the instance buffer, textured by
/// `page`'s `layer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DrawItem {
    pub(crate) page: PageId,
    pub(crate) layer: LayerKind,
}

/// What the compositor currently holds for a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PageLayersInfo {
    /// Which held view frame is drawn: [`LayerKind::View`] or
    /// [`LayerKind::KeptView`]. The fields below describe that one.
    pub(crate) view: LayerKind,
    /// Size of the view texture, in texels.
    pub(crate) view_size: PixelSize,
    /// The CSS size of the view the texture shows.
    pub(crate) view_css: CssSize,
    /// Whether the view texture came from a CPU upload.
    pub(crate) view_is_cpu: bool,
    /// Placement of a visible, painted popup in view texels.
    pub(crate) popup: Option<PixelRect>,
}

/// Counters produced while building the draw list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct DrawCounts {
    pub(crate) pages_without_texture: u32,
    pub(crate) cpu_textures: u32,
}

/// The quad for a page's visible popup, given where the page's view is drawn
/// in canvas space. The popup's placement is in view texels.
pub(crate) fn popup_quad(origin: Vec2, size: Vec2, info: &PageLayersInfo) -> Option<QuadInstance> {
    let popup = info.popup.filter(|_| !info.view_size.is_empty())?;
    let texel_to_canvas =
        size / Vec2::new(info.view_size.width as f32, info.view_size.height as f32);
    Some(QuadInstance::new(
        origin + Vec2::new(popup.x as f32, popup.y as f32) * texel_to_canvas,
        Vec2::new(popup.width as f32, popup.height as f32) * texel_to_canvas,
        0.0,
    ))
}
