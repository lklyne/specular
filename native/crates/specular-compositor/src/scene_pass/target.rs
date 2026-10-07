//! The multisampled colour attachment the scene pass draws into.

use crate::pipeline::SCENE_SAMPLES;

/// A multisampled texture matching the target, rebuilt when the target's
/// size changes.
#[derive(Debug)]
pub(crate) struct MultisampledTarget {
    view: wgpu::TextureView,
    size: wgpu::Extent3d,
}

impl MultisampledTarget {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat, size: wgpu::Extent3d) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene-multisampled"),
            size,
            mip_level_count: 1,
            sample_count: SCENE_SAMPLES,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            size,
        }
    }

    /// The attachment for a target of `size`, reusing `slot`'s when it fits.
    pub(crate) fn for_size<'a>(
        slot: &'a mut Option<Self>,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: wgpu::Extent3d,
    ) -> &'a wgpu::TextureView {
        let stale = slot.as_ref().is_none_or(|target| target.size != size);
        if stale {
            *slot = None;
        }
        &slot
            .get_or_insert_with(|| Self::new(device, format, size))
            .view
    }
}
