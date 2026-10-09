//! Turning CEF paint callbacks into [`FrameEvent`]s.

use std::time::Instant;

use cef::{PaintElementType, Rect};
use specular_core::{CssSize, FrameEvent, FrameLayer, PageFrame, PixelRect, PixelSize};

use crate::coords::rect_from_cef;
use crate::cpu_frame::copy_paint;
use crate::page::PageContext;

/// The layer a paint of `texels` targets and the CSS size of the view it
/// belongs to, or `None` for a popup paint that arrives before `OnPopupSize`
/// placed it (nothing to draw it against yet).
fn layer_for(
    ctx: &PageContext,
    kind: PaintElementType,
    texels: PixelSize,
) -> Option<(FrameLayer, CssSize)> {
    let mut geometry = ctx.geometry();
    if kind == PaintElementType::POPUP {
        let rect = geometry.popup()?;
        Some((FrameLayer::Popup { rect }, geometry.viewport))
    } else {
        Some((FrameLayer::View, geometry.frame_viewport(texels)))
    }
}

fn dirty_rects(dirty: Option<&[Rect]>) -> Vec<PixelRect> {
    dirty
        .unwrap_or_default()
        .iter()
        .map(|r| rect_from_cef(r.x, r.y, r.width, r.height))
        .collect()
}

/// `OnPaint`: copies the CPU buffer out (it dies with the callback).
///
/// `bytes` is the callback's whole BGRA buffer, `width * height * 4` long.
pub(crate) fn on_paint(
    ctx: &PageContext,
    kind: PaintElementType,
    dirty: Option<&[Rect]>,
    bytes: &[u8],
    width: i32,
    height: i32,
) {
    let produced_at = Instant::now();
    let texels = PixelSize::new(width.max(0) as u32, height.max(0) as u32);
    let Some((layer, viewport)) = layer_for(ctx, kind, texels) else {
        return;
    };
    match copy_paint(bytes, width, height, &dirty_rects(dirty)) {
        Ok(frame) => ctx.push(specular_core::PageEvent::Frame(FrameEvent {
            page: ctx.id,
            layer,
            viewport,
            frame: PageFrame::Cpu(frame),
            produced_at,
        })),
        Err(err) => tracing::warn!(page = %ctx.id, %err, "dropping CPU paint"),
    }
}

/// `OnAcceleratedPaint`: retains the IOSurface as a zero-copy frame.
#[cfg(target_os = "macos")]
pub(crate) fn on_accelerated_paint(
    ctx: &PageContext,
    kind: PaintElementType,
    info: Option<&cef::AcceleratedPaintInfo>,
) {
    use specular_core::PixelFormat;

    let produced_at = Instant::now();
    let Some(info) = info else {
        return;
    };
    let format = if info.format == cef::ColorType::RGBA_8888 {
        PixelFormat::Rgba8Unorm
    } else if info.format == cef::ColorType::BGRA_8888 {
        PixelFormat::Bgra8Unorm
    } else {
        tracing::warn!(page = %ctx.id, format = ?info.format, "unsupported shared texture format");
        return;
    };
    let coded = &info.extra.coded_size;
    let size = PixelSize::new(coded.width.max(0) as u32, coded.height.max(0) as u32);
    let visible = &info.extra.visible_rect;
    if visible.x != 0
        || visible.y != 0
        || visible.width != coded.width
        || visible.height != coded.height
    {
        // SharedTexture carries no sub-rect, so padded surfaces would be
        // sampled whole; logged so a real run shows whether it happens.
        tracing::debug!(page = %ctx.id, ?coded, ?visible, "shared texture visible rect differs from coded size");
    }
    let Some(surface) = std::ptr::NonNull::new(info.shared_texture_io_surface) else {
        return;
    };
    if size.is_empty() {
        return;
    }
    let Some((layer, viewport)) = layer_for(ctx, kind, size) else {
        return;
    };
    let Some(lease) = ctx.frames.try_lease() else {
        tracing::debug!(page = %ctx.id, "texture cap reached, dropping paint");
        ctx.push(specular_core::PageEvent::FrameDropped { page: ctx.id });
        return;
    };
    // SAFETY: `surface` is this callback's `shared_texture_io_surface`, valid
    // until the callback returns, and we are inside that callback.
    let texture = unsafe { crate::iosurface::retain_shared_texture(surface, size, format, lease) };
    ctx.push(specular_core::PageEvent::Frame(FrameEvent {
        page: ctx.id,
        layer,
        viewport,
        frame: PageFrame::GpuShared(texture),
        produced_at,
    }));
}
