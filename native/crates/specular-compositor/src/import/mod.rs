//! Zero-copy import of producer GPU surfaces into wgpu textures.
//!
//! The representative path. Only macOS (IOSurface -> `MTLTexture` -> wgpu-hal
//! Metal texture) is implemented; elsewhere sources fall back to CPU frames.

#[cfg(target_os = "macos")]
mod iosurface;

use specular_core::{NativeSurface, SharedTexture};

/// Wraps `shared`'s surface as a sampled wgpu texture of `format` without
/// copying. The caller must keep `shared` alive until the GPU has finished
/// with the returned texture.
pub(crate) fn import_shared(
    device: &wgpu::Device,
    shared: &SharedTexture,
    format: wgpu::TextureFormat,
) -> Result<wgpu::Texture, String> {
    match shared.surface() {
        #[cfg(target_os = "macos")]
        NativeSurface::IoSurface(surface) => {
            iosurface::import(device, surface, shared.size(), format)
        }
        #[cfg(not(target_os = "macos"))]
        NativeSurface::IoSurface(_) => {
            let _ = (device, format);
            Err("IOSurface frames can only be imported on macOS".to_owned())
        }
        other => Err(format!("unsupported native surface {other:?}")),
    }
}
