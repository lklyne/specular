//! The multisampled colour attachment the scene pass draws into.

use crate::pipeline::SCENE_SAMPLES;

/// A multisampled texture matching the target, rebuilt when the target's
/// size changes.
#[derive(Debug)]
pub(crate) struct MultisampledTarget {
    view: wgpu::TextureView,
    size: wgpu::Extent3d,
}

/// The attachment's descriptor. It is only ever cleared, drawn into and
/// resolved inside one pass, so it is marked transient: on Apple GPUs it
/// then lives in tile memory and takes none of the app's (sixteen bytes a
/// pixel otherwise, about 100 MB for a 1600x1000 window at 2x).
fn descriptor(
    format: wgpu::TextureFormat,
    size: wgpu::Extent3d,
) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some("scene-multisampled"),
        size,
        mip_level_count: 1,
        sample_count: SCENE_SAMPLES,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TRANSIENT_ATTACHMENT,
        view_formats: &[],
    }
}

impl MultisampledTarget {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat, size: wgpu::Extent3d) -> Self {
        let texture = device.create_texture(&descriptor(format, size));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_multisampled_attachment_is_transient_and_nothing_else() {
        let size = wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        };
        let usage = descriptor(wgpu::TextureFormat::Bgra8Unorm, size).usage;
        // Any other usage would make the texture take memory again.
        assert_eq!(
            usage,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TRANSIENT_ATTACHMENT
        );
    }
}
