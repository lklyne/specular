//! Retaining CEF's `OnAcceleratedPaint` IOSurface past the callback (macOS).
//!
//! The only unsafe IOSurface code in the crate. Importing the surface into
//! wgpu (`newTextureWithDescriptor:iosurface:plane:` -> wgpu-hal Metal
//! `texture_from_raw` -> `create_texture_from_hal`) is the compositor's job,
//! so this crate never touches Metal or wgpu.
//!
//! # Copy or retain
//!
//! CEF documents the handle as valid only during the callback: "the contents
//! should be copied to a texture owned by the client". A copy needs a GPU
//! device inside the callback, and the page source deliberately has none,
//! so the spike **retains** instead, the same zero-copy model Electron's
//! `texture.release()` OSR API exposes:
//!
//! 1. `CFRetain` keeps the IOSurface object alive even if Chromium frees its
//!    pool slot (on resize the pool is rebuilt).
//! 2. `IOSurfaceIncrementUseCount` marks it in use. Chromium's capture frame
//!    pool checks `IOSurfaceIsInUse` before recycling a buffer, so it
//!    allocates a fresh surface instead of painting into one the compositor
//!    is still sampling.
//! 3. The [`SharedTexture`] release closure undoes both when the compositor
//!    drops the frame, and frees the page's [`FrameLease`], which caps how
//!    many surfaces one page can pin ([`specular_core::MAX_OUTSTANDING_TEXTURES`]).
//!
//! Point 2 relies on Chromium internals, not CEF's documented contract. If a
//! real run shows tearing (the compositor sampling a surface Chromium is
//! repainting), fall back to the documented strategy: give the source a
//! `wgpu::Device` and blit into a compositor-owned texture inside the
//! callback. That is a `specular-core` contract change and costs one GPU copy
//! per frame.
//!
//! Releasing after `cef_shutdown` is fine: both calls are plain
//! CoreFoundation/IOSurface API and do not touch CEF.

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_core_foundation::CFRetained;
use objc2_io_surface::IOSurfaceRef;
use specular_core::{NativeSurface, PixelFormat, PixelSize, SharedTexture};

use crate::pool::FrameLease;

/// Retains CEF's IOSurface and wraps it as a [`SharedTexture`] whose drop
/// returns it to Chromium's pool.
///
/// # Safety
///
/// `surface` must be the `shared_texture_io_surface` of the
/// `cef_accelerated_paint_info_t` passed to the `OnAcceleratedPaint` call
/// currently executing on this thread: a valid `IOSurfaceRef` for the
/// duration of that call.
pub(crate) unsafe fn retain_shared_texture(
    surface: NonNull<c_void>,
    size: PixelSize,
    format: PixelFormat,
    lease: FrameLease,
) -> SharedTexture {
    // SAFETY: the caller guarantees `surface` is a live IOSurfaceRef for the
    // current callback; CFRetain takes our own +1 so it outlives the callback.
    let retained: CFRetained<IOSurfaceRef> = unsafe { CFRetained::retain(surface.cast()) };
    retained.increment_use_count();
    SharedTexture::new(NativeSurface::IoSurface(surface), size, format, move || {
        retained.decrement_use_count();
        drop(retained);
        drop(lease);
    })
}
