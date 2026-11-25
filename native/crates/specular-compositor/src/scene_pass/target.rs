//! The multisampled colour attachment the scene pass draws into.

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
    samples: u32,
    size: wgpu::Extent3d,
) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some("scene-multisampled"),
        size,
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TRANSIENT_ATTACHMENT,
        view_formats: &[],
    }
}

impl MultisampledTarget {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        samples: u32,
        size: wgpu::Extent3d,
    ) -> Self {
        let texture = device.create_texture(&descriptor(format, samples, size));
        Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            size,
        }
    }

    /// What the scene pass draws into for `target`: above one sample, `slot`'s
    /// texture resolved into the target and then discarded; at one, the
    /// target itself.
    pub(crate) fn attachment<'a>(
        slot: &'a mut Option<Self>,
        (device, format, samples): (&wgpu::Device, wgpu::TextureFormat, u32),
        size: wgpu::Extent3d,
        target: &'a wgpu::TextureView,
    ) -> (
        &'a wgpu::TextureView,
        Option<&'a wgpu::TextureView>,
        wgpu::StoreOp,
    ) {
        if samples > 1 {
            let view = Self::for_size(slot, device, format, samples, size);
            (view, Some(target), wgpu::StoreOp::Discard)
        } else {
            (target, None, wgpu::StoreOp::Store)
        }
    }

    /// The attachment for a target of `size`, reusing `slot`'s when it fits.
    fn for_size<'a>(
        slot: &'a mut Option<Self>,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        samples: u32,
        size: wgpu::Extent3d,
    ) -> &'a wgpu::TextureView {
        let stale = slot.as_ref().is_none_or(|target| target.size != size);
        if stale {
            *slot = None;
        }
        &slot
            .get_or_insert_with(|| Self::new(device, format, samples, size))
            .view
    }
}
