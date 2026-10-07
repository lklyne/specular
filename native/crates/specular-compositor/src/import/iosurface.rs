//! IOSurface -> `MTLTexture` -> wgpu texture, sharing the surface's memory.

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_io_surface::IOSurfaceRef;
use objc2_metal::{
    MTLDevice as _, MTLPixelFormat, MTLStorageMode, MTLTextureDescriptor, MTLTextureType,
    MTLTextureUsage,
};
use specular_core::PixelSize;
use wgpu::hal::api::Metal;

use crate::error::FrameImportError;
use crate::upload::extent;

fn metal_pixel_format(format: wgpu::TextureFormat) -> Result<MTLPixelFormat, FrameImportError> {
    match format {
        wgpu::TextureFormat::Bgra8Unorm => Ok(MTLPixelFormat::BGRA8Unorm),
        wgpu::TextureFormat::Bgra8UnormSrgb => Ok(MTLPixelFormat::BGRA8Unorm_sRGB),
        wgpu::TextureFormat::Rgba8Unorm => Ok(MTLPixelFormat::RGBA8Unorm),
        wgpu::TextureFormat::Rgba8UnormSrgb => Ok(MTLPixelFormat::RGBA8Unorm_sRGB),
        other => Err(FrameImportError::UnsupportedFormat(other)),
    }
}

/// Imports plane 0 of `surface` (a live `IOSurfaceRef` whose use count the
/// producer holds) as a `size` texture of `format`.
pub(super) fn import(
    device: &wgpu::Device,
    surface: NonNull<c_void>,
    size: PixelSize,
    format: wgpu::TextureFormat,
) -> Result<wgpu::Texture, FrameImportError> {
    if size.is_empty() {
        return Err(FrameImportError::EmptyFrame {
            width: size.width,
            height: size.height,
        });
    }
    let descriptor = wgpu::TextureDescriptor {
        label: Some("page-iosurface"),
        size: extent(size),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let metal_descriptor = MTLTextureDescriptor::new();
    metal_descriptor.setTextureType(MTLTextureType::Type2D);
    metal_descriptor.setPixelFormat(metal_pixel_format(format)?);
    metal_descriptor.setUsage(MTLTextureUsage::ShaderRead);
    metal_descriptor.setStorageMode(MTLStorageMode::Managed);
    // SAFETY: plain property setters on a descriptor we own; the dimensions
    // are non-zero (checked above) and within IOSurface limits by origin.
    unsafe { metal_descriptor.setWidth(size.width as usize) };
    // SAFETY: as above.
    unsafe { metal_descriptor.setHeight(size.height as usize) };

    // SAFETY: the producer incremented the surface's use count and the
    // `SharedTexture` that owns this pointer outlives this call, so it points
    // at a live IOSurface for the borrow's duration.
    let io_surface: &IOSurfaceRef = unsafe { surface.cast::<IOSurfaceRef>().as_ref() };

    let raw = {
        // SAFETY: the hal device is only used to create one texture inside
        // this block and is dropped before wgpu is used again.
        let hal_device = unsafe { device.as_hal::<Metal>() }.ok_or(FrameImportError::NotMetal)?;
        hal_device
            .raw_device()
            .newTextureWithDescriptor_iosurface_plane(&metal_descriptor, io_surface, 0)
            .ok_or(FrameImportError::MetalRefused)?
    };

    // SAFETY: `raw` is a 2D, single-layer, single-mip texture created on this
    // device's `MTLDevice` with the pixel format matching `format` and the
    // given size, which is exactly what we describe to wgpu-hal.
    let hal_texture = unsafe {
        <Metal as wgpu::hal::Api>::Device::texture_from_raw(
            raw,
            format,
            MTLTextureType::Type2D,
            1,
            1,
            wgpu::hal::CopyExtent {
                width: size.width,
                height: size.height,
                depth: 1,
            },
            None,
        )
    };
    // SAFETY: `hal_texture` was created from this device's Metal device and
    // matches `descriptor`; it starts in the shader-readable state.
    Ok(unsafe {
        device.create_texture_from_hal::<Metal>(
            hal_texture,
            &descriptor,
            wgpu::TextureUses::RESOURCE,
        )
    })
}

/// The system-wide ID of `surface` (a live `IOSurfaceRef` the caller's
/// [`SharedTexture`](specular_core::SharedTexture) retains).
pub(super) fn surface_id(surface: NonNull<c_void>) -> u64 {
    // SAFETY: the `SharedTexture` that owns this pointer retains the
    // IOSurface and outlives this call.
    let io_surface: &IOSurfaceRef = unsafe { surface.cast::<IOSurfaceRef>().as_ref() };
    u64::from(io_surface.id())
}
