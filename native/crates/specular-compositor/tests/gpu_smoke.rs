//! Renders one empty frame offscreen. Skips (passes) on machines with no GPU
//! adapter, which includes the Linux CI runner and the dev container.

use glam::Vec2;
use specular_compositor::{Compositor, CompositorError, DotGrid, GpuContext, SceneView};
use specular_core::Camera;

#[test]
fn render_empty_scene_succeeds_when_adapter_present() {
    let gpu = match pollster::block_on(GpuContext::headless()) {
        Ok(gpu) => gpu,
        Err(CompositorError::NoAdapter(reason)) => {
            eprintln!("skipping: no GPU adapter ({reason})");
            return;
        }
        Err(other) => panic!("GPU setup failed: {other}"),
    };
    let format = wgpu::TextureFormat::Bgra8UnormSrgb;
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("smoke-target"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut compositor = Compositor::new(gpu.device.clone(), gpu.queue.clone(), format);
    let stats = compositor.render(
        &view,
        &SceneView {
            camera: Camera::default(),
            viewport: Vec2::new(64.0, 64.0),
            scale_factor: 1.0,
            pages: &[],
            grid: DotGrid::default(),
        },
    );
    assert_eq!(stats.pages_drawn, 0);
}
