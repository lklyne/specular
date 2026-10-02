//! Zero-copy import of producer GPU surfaces into wgpu textures.
//!
//! The representative path. Only macOS (IOSurface -> `MTLTexture` -> wgpu-hal
//! Metal texture) is implemented; elsewhere sources fall back to CPU frames.

#[cfg(target_os = "macos")]
mod iosurface;

use specular_core::{NativeSurface, SharedTexture};

use crate::error::FrameImportError;

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
