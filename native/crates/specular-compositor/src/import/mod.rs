//! Zero-copy import of producer GPU surfaces into wgpu textures.
//!
//! The representative path. Only macOS (IOSurface -> `MTLTexture` -> wgpu-hal
//! Metal texture) is implemented; elsewhere sources fall back to CPU frames.

#[cfg(target_os = "macos")]
mod iosurface;

use specular_core::{NativeSurface, SharedTexture};

use crate::error::FrameImportError;

/// A number naming the surface behind `shared` for as long as it is retained:
/// the same surface gives the same number on every paint.
///
/// Not the handle's address: each paint can arrive on a new handle object
/// for a surface seen before, and only the system-wide IOSurface ID repeats.
pub(crate) fn surface_identity(shared: &SharedTexture) -> u64 {
    match shared.surface() {
        #[cfg(target_os = "macos")]
        NativeSurface::IoSurface(surface) => iosurface::surface_id(surface),
        #[cfg(not(target_os = "macos"))]
        NativeSurface::IoSurface(surface) => surface.as_ptr() as usize as u64,
        _ => 0,
    }
}

/// Wraps `shared`'s surface as a sampled wgpu texture of `format` without
/// copying. The caller must keep `shared` alive until the GPU has finished
/// with the returned texture.
pub(crate) fn import_shared(
    device: &wgpu::Device,
    shared: &SharedTexture,
    format: wgpu::TextureFormat,
) -> Result<wgpu::Texture, FrameImportError> {
    match shared.surface() {
        #[cfg(target_os = "macos")]
        NativeSurface::IoSurface(surface) => {
            iosurface::import(device, surface, shared.size(), format)
        }
        #[cfg(not(target_os = "macos"))]
        NativeSurface::IoSurface(_) => {
            let _ = (device, format);
            Err(FrameImportError::UnsupportedSurface)
        }
        _ => Err(FrameImportError::UnsupportedSurface),
    }
}
