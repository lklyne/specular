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
    /// A shared surface could not be imported (wrong backend, bad handle,
    /// unsupported pixel format).
    #[error("failed to import shared texture for {page}: {reason}")]
    Import {
        /// The page whose frame failed.
        page: PageId,
        /// Why.
        reason: String,
    },
}
