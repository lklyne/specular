//! Adapter/device acquisition shared by the app, bench and smoke tests.

use crate::error::CompositorError;

/// An initialised wgpu instance, adapter, device and queue.
#[derive(Debug)]
pub struct GpuContext {
    /// The instance (needed to create window surfaces).
    pub instance: wgpu::Instance,
    /// The chosen adapter.
    pub adapter: wgpu::Adapter,
    /// The device (cheaply cloneable handle).
    pub device: wgpu::Device,
    /// The queue (cheaply cloneable handle).
    pub queue: wgpu::Queue,
}

impl GpuContext {
    /// Acquires a high-performance adapter compatible with `surface` (or any
    /// adapter when headless) and creates a device on it.
    ///
    /// Returns [`CompositorError::NoAdapter`] on machines without a usable GPU.
    pub async fn new(
        instance: wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self, CompositorError> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface,
                ..wgpu::RequestAdapterOptions::default()
            })
            .await
            .map_err(CompositorError::NoAdapter)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("specular-compositor"),
                ..wgpu::DeviceDescriptor::default()
            })
            .await?;
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    /// [`new`](Self::new) with a default instance and no surface.
    pub async fn headless() -> Result<Self, CompositorError> {
        Self::new(wgpu::Instance::default(), None).await
    }
}
