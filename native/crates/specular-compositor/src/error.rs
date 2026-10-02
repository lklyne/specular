//! Compositor errors.

use specular_core::PageId;

/// Errors from GPU setup or frame ingestion.
#[derive(Debug, thiserror::Error)]
pub enum CompositorError {
    /// No GPU adapter is available (headless CI, no Metal/Vulkan). Callers and
    /// smoke tests treat this as "skip", not failure.
    #[error("no compatible GPU adapter")]
    NoAdapter(#[source] wgpu::RequestAdapterError),
    /// The adapter refused to create a device.
    #[error("failed to create GPU device")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    /// A page frame could not be turned into a texture.
    #[error("failed to import a frame for {page}")]
    Import {
        /// The page whose frame failed.
        page: PageId,
        /// Why.
        #[source]
        source: FrameImportError,
    },
}

/// Why a page frame was rejected before it reached wgpu (whose validation
/// failures abort the process instead of returning an error).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrameImportError {
    /// The frame has no texels.
    #[error("empty frame {width}x{height}")]
    EmptyFrame {
        /// Width in texels.
        width: u32,
        /// Height in texels.
        height: u32,
    },
    /// The frame is larger than the device's 2D texture limit.
    #[error("frame {width}x{height} exceeds the device limit of {max}")]
    ExceedsDeviceLimit {
        /// Width in texels.
        width: u32,
        /// Height in texels.
        height: u32,
        /// `max_texture_dimension_2d`.
        max: u32,
    },
    /// A CPU frame's rows are shorter than its width.
    #[error("stride {stride} is shorter than a {width}px row")]
    ShortStride {
        /// Bytes per row.
        stride: u32,
        /// Width in texels.
        width: u32,
    },
    /// A CPU frame holds fewer bytes than its size needs.
    #[error("frame has {actual} bytes, {needed} needed")]
    ShortBuffer {
        /// Bytes present.
        actual: u64,
        /// Bytes needed.
        needed: u64,
    },
    /// A native surface kind this platform cannot import (an `IOSurface`
    /// imports only on macOS).
    #[error("this platform cannot import the frame's native surface")]
    UnsupportedSurface,
    /// No Metal pixel format matches the texture format.
    #[error("no IOSurface import for {0:?}")]
    UnsupportedFormat(wgpu::TextureFormat),
    /// Shared surfaces need the Metal backend.
    #[error("wgpu is not running on Metal")]
    NotMetal,
    /// `newTextureWithDescriptor:iosurface:plane:` returned nil.
    #[error("Metal refused to wrap the IOSurface")]
    MetalRefused,
}
