//! Painted output of a page host.
//!
//! A page paints either a GPU shared surface (the representative, zero-copy
//! path: CEF `OnAcceleratedPaint` -> IOSurface on macOS) or CPU BGRA bytes
//! (CEF `OnPaint`; non-representative, kept for platforms without the
//! shared-texture path and for the synthetic source). ADR 0038 measured that
//! a CPU/JPEG path collapses to ~4-8 fps under animation, so benchmark results
//! from [`PageFrame::Cpu`] must never be compared against Electron.

use std::ffi::c_void;
use std::fmt;
use std::ptr::NonNull;
use std::time::Instant;

use crate::geometry::{PixelRect, PixelSize};
use crate::page::PageId;

/// Per-page cap on [`SharedTexture`]s alive at once, mirroring Electron's
/// `MAX_OUTSTANDING_TEXTURES` (ADR 0038). A source that hits the cap drops new
/// frames rather than queueing them, so a slow compositor sheds load instead
/// of falling arbitrarily far behind.
pub const MAX_OUTSTANDING_TEXTURES: usize = 6;

/// Texel layout of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PixelFormat {
    /// 8-bit BGRA, premultiplied alpha (CEF `CEF_COLOR_TYPE_BGRA_8888`).
    #[default]
    Bgra8Unorm,
    /// 8-bit RGBA, premultiplied alpha (CEF `CEF_COLOR_TYPE_RGBA_8888`).
    Rgba8Unorm,
}

impl PixelFormat {
    /// Bytes per texel.
    pub const fn bytes_per_pixel(self) -> u32 {
        4
    }
}

/// A platform GPU surface handle. Only the compositor dereferences it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeSurface {
    /// A macOS `IOSurfaceRef`. The producer has already called
    /// `IOSurfaceIncrementUseCount` (so Chromium's pool will not recycle it)
    /// and the [`SharedTexture`]'s release callback decrements it.
    IoSurface(NonNull<c_void>),
}

type ReleaseFn = Box<dyn FnOnce()>;

/// A GPU surface painted by a page host, held until the compositor drops it.
///
/// Dropping a `SharedTexture` runs its release callback exactly once, which
/// returns the surface to the producer's pool. Holding many of them is what
/// [`MAX_OUTSTANDING_TEXTURES`] bounds. Deliberately `!Send`: CEF and the
/// window event loop share the main thread on macOS, and the handle must be
/// released on the thread that produced it.
pub struct SharedTexture {
    surface: NativeSurface,
    size: PixelSize,
    format: PixelFormat,
    release: Option<ReleaseFn>,
}

impl SharedTexture {
    /// Wraps a retained surface. `release` runs once when the texture drops.
    pub fn new(
        surface: NativeSurface,
        size: PixelSize,
        format: PixelFormat,
        release: impl FnOnce() + 'static,
    ) -> Self {
        Self {
            surface,
            size,
            format,
            release: Some(Box::new(release)),
        }
    }

    /// The platform handle.
    pub fn surface(&self) -> NativeSurface {
        self.surface
    }

    /// Size in texels.
    pub fn size(&self) -> PixelSize {
        self.size
    }

    /// Texel layout.
    pub fn format(&self) -> PixelFormat {
        self.format
    }
}

impl fmt::Debug for SharedTexture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedTexture")
            .field("surface", &self.surface)
            .field("size", &self.size)
            .field("format", &self.format)
            .finish_non_exhaustive()
    }
}

impl Drop for SharedTexture {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            release();
        }
    }
}

/// CPU-painted frame: tightly described BGRA bytes copied out of `OnPaint`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CpuFrame {
    /// Size in texels.
    pub size: PixelSize,
    /// Bytes per row (`>= size.width * 4`).
    pub stride: u32,
    /// Premultiplied BGRA texels, `stride * size.height` bytes.
    pub bgra: Vec<u8>,
    /// Regions that changed since the previous frame of this layer; empty
    /// means the whole frame is dirty.
    pub dirty: Vec<PixelRect>,
}

/// Pixels for one paint of one layer.
#[derive(Debug)]
pub enum PageFrame {
    /// Zero-copy GPU surface (representative path).
    GpuShared(SharedTexture),
    /// CPU bytes (non-representative fallback).
    Cpu(CpuFrame),
}

impl PageFrame {
    /// Size of the frame in texels.
    pub fn size(&self) -> PixelSize {
        match self {
            Self::GpuShared(texture) => texture.size(),
            Self::Cpu(frame) => frame.size,
        }
    }

    /// Whether this frame came through the representative zero-copy path.
    pub fn is_representative(&self) -> bool {
        matches!(self, Self::GpuShared(_))
    }
}

/// Which surface of a page a frame paints (CEF `PET_VIEW` / `PET_POPUP`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameLayer {
    /// The page's main view, covering its whole viewport.
    View,
    /// A popup widget (`<select>` list, date picker, autofill) drawn over the
    /// view at `rect`, in the view frame's texel space (CEF `OnPopupSize`).
    Popup {
        /// Placement within the view frame.
        rect: PixelRect,
    },
}

/// One painted frame delivered by a [`PageSource`](crate::PageSource).
#[derive(Debug)]
pub struct FrameEvent {
    /// The page that painted.
    pub page: PageId,
    /// Which layer the pixels belong to.
    pub layer: FrameLayer,
    /// The pixels.
    pub frame: PageFrame,
    /// When the source received the paint, for paint-to-present latency.
    pub produced_at: Instant,
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    #[test]
    fn dropping_shared_texture_runs_release_once() {
        let released = Rc::new(Cell::new(0));
        let counter = Rc::clone(&released);
        let texture = SharedTexture::new(
            NativeSurface::IoSurface(NonNull::dangling()),
            PixelSize::new(4, 4),
            PixelFormat::Bgra8Unorm,
            move || counter.set(counter.get() + 1),
        );
        drop(texture);
        assert_eq!(released.get(), 1);
    }

    #[test]
    fn cpu_frame_is_not_representative() {
        assert!(!PageFrame::Cpu(CpuFrame::default()).is_representative());
    }
}
