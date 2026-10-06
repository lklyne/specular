//! Offscreen renders checked pixel by pixel. Each test passes with a printed
//! skip on machines with no GPU adapter (the Linux CI runner).

mod common;

use std::time::Instant;

use common::{TARGET_SIZE, gpu_or_skip, pixel, read_pixels, render_target};
use glam::Vec2;
use specular_compositor::{
    Compositor, CompositorError, DotGrid, FrameImportError, PageDraw, RenderStats, SceneView,
    ShapeDraw, ShapeExtent,
};
use specular_core::{
    Camera, CanvasRect, CpuFrame, FrameEvent, FrameLayer, PageEvent, PageFrame, PageId, PixelRect,
    PixelSize,
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const PAGE: PageId = PageId(1);
const PAGE_RECT: CanvasRect = CanvasRect::new(8.0, 8.0, 48.0, 48.0);
/// Linear blue; encodes to exactly (0, 0, 255).
const BACKGROUND: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const BACKGROUND_TEXEL: [u8; 4] = [0, 0, 255, 255];

fn plain_grid() -> DotGrid {
    DotGrid {
        spacing: 0.0,
        background: BACKGROUND,
        ..DotGrid::default()
    }
}

fn solid_frame(size: PixelSize, bgra: [u8; 4]) -> CpuFrame {
    CpuFrame {
        size,
        stride: size.width * 4,
        bgra: bgra.repeat(size.area() as usize),
        dirty: Vec::new(),
    }
}

fn frame_event(layer: FrameLayer, frame: CpuFrame) -> PageEvent {
    PageEvent::Frame(FrameEvent {
        page: PAGE,
        layer,
        frame: PageFrame::Cpu(frame),
        produced_at: Instant::now(),
    })
}

struct Harness {
    gpu: specular_compositor::GpuContext,
    compositor: Compositor,
    target: wgpu::Texture,
}

impl Harness {
    fn new() -> Option<Self> {
        let gpu = gpu_or_skip()?;
        let compositor = Compositor::new(gpu.device.clone(), gpu.queue.clone(), FORMAT);
        let target = render_target(&gpu, FORMAT);
        Some(Self {
            gpu,
            compositor,
            target,
        })
    }

    fn render(&mut self, camera: Camera, grid: DotGrid, pages: &[PageDraw]) -> RenderStats {
        self.render_with_shapes(camera, grid, pages, &[])
    }

    fn render_with_shapes(
        &mut self,
        camera: Camera,
        grid: DotGrid,
        pages: &[PageDraw],
        shapes: &[ShapeDraw],
    ) -> RenderStats {
        let view = self
            .target
            .create_view(&wgpu::TextureViewDescriptor::default());
        let size = TARGET_SIZE as f32;
        self.compositor.render(
            &view,
            &SceneView {
                camera,
                viewport: Vec2::new(size, size),
                scale_factor: 1.0,
                pages,
                shapes,
                grid,
            },
        )
    }

    fn render_page(&mut self) -> Vec<[u8; 4]> {
        let pages = [PageDraw {
            page: PAGE,
            rect: PAGE_RECT,
        }];
        self.render(Camera::default(), plain_grid(), &pages);
        read_pixels(&self.gpu, &self.target)
    }

    fn send_view(&mut self, frame: CpuFrame) {
        let result = self
            .compositor
            .handle_page_event(frame_event(FrameLayer::View, frame));
        assert!(result.is_ok(), "frame rejected: {result:?}");
    }
}

#[test]
fn render_empty_scene_fills_background() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    harness.render(Camera::default(), plain_grid(), &[]);
    let pixels = read_pixels(&harness.gpu, &harness.target);
    assert_eq!(pixel(&pixels, 40, 3), BACKGROUND_TEXEL);
}

#[test]
fn grid_draws_a_dot_at_the_world_origin() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let grid = DotGrid {
        dot: [1.0, 0.0, 0.0, 1.0],
        ..plain_grid()
    };
    let grid = DotGrid {
        spacing: 20.0,
        ..grid
    };
    // Pan puts the world origin on pixel (10, 10)'s centre.
    harness.render(Camera::new(Vec2::splat(10.5), 1.0), grid, &[]);
    let pixels = read_pixels(&harness.gpu, &harness.target);
    assert_eq!(
        [pixel(&pixels, 10, 10), pixel(&pixels, 20, 10)],
        [[255, 0, 0, 255], BACKGROUND_TEXEL]
    );
}

#[test]
fn page_without_frame_counts_as_missing_texture() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let pages = [PageDraw {
        page: PAGE,
        rect: PAGE_RECT,
    }];
    let stats = harness.render(Camera::default(), plain_grid(), &pages);
    assert_eq!(stats.pages_without_texture, 1);
}

#[test]
fn cpu_frame_is_drawn_inside_page_rect() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    harness.send_view(solid_frame(PixelSize::new(48, 48), [10, 20, 200, 255]));
    let pixels = harness.render_page();
    assert_eq!(pixel(&pixels, 32, 32), [200, 20, 10, 255]);
}

#[test]
fn rounded_corner_reveals_background() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    harness.send_view(solid_frame(PixelSize::new(48, 48), [10, 20, 200, 255]));
    let pixels = harness.render_page();
    assert_eq!(pixel(&pixels, 8, 8), BACKGROUND_TEXEL);
}

#[test]
fn shape_fill_covers_its_rect_and_leaves_the_rest() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let shapes = [ShapeDraw {
        extent: ShapeExtent::Canvas(CanvasRect::new(16.0, 16.0, 32.0, 32.0)),
        corner_radius: 0.0,
        fill: [1.0, 0.0, 0.0, 1.0],
        stroke: [0.0; 4],
        stroke_width: 0.0,
    }];
    let stats = harness.render_with_shapes(Camera::default(), plain_grid(), &[], &shapes);
    let pixels = read_pixels(&harness.gpu, &harness.target);
    assert_eq!(
        (
            pixel(&pixels, 32, 32),
            pixel(&pixels, 4, 4),
            stats.shapes_drawn
        ),
        ([255, 0, 0, 255], BACKGROUND_TEXEL, 1)
    );
}

#[test]
fn shape_stroke_sits_outside_the_rect_edge() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let shapes = [ShapeDraw {
        extent: ShapeExtent::Canvas(CanvasRect::new(16.0, 16.0, 32.0, 32.0)),
        corner_radius: 0.0,
        fill: [1.0, 0.0, 0.0, 1.0],
        stroke: [0.0, 1.0, 0.0, 1.0],
        stroke_width: 2.0,
    }];
    harness.render_with_shapes(Camera::default(), plain_grid(), &[], &shapes);
    let pixels = read_pixels(&harness.gpu, &harness.target);
    // Pixel 15 is the first outside the rect; 17 is inside it, 13 is clear.
    assert_eq!(
        [
            pixel(&pixels, 32, 15),
            pixel(&pixels, 32, 16),
            pixel(&pixels, 32, 13)
        ],
        [[0, 255, 0, 255], [255, 0, 0, 255], BACKGROUND_TEXEL]
    );
}

#[test]
fn screen_sized_shape_keeps_its_size_when_zoomed() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // Zoom 0.5 puts canvas (64, 64) at pixel (32, 32).
    let shapes = [ShapeDraw {
        extent: ShapeExtent::Screen {
            anchor: glam::Vec2::new(64.0, 64.0),
            size: glam::Vec2::splat(16.0),
        },
        corner_radius: 8.0,
        fill: [1.0, 0.0, 0.0, 1.0],
        stroke: [0.0; 4],
        stroke_width: 0.0,
    }];
    harness.render_with_shapes(Camera::new(Vec2::ZERO, 0.5), plain_grid(), &[], &shapes);
    let pixels = read_pixels(&harness.gpu, &harness.target);
    // 6 px from the centre is inside a 16 px circle; 12 px is outside.
    assert_eq!(
        [pixel(&pixels, 38, 32), pixel(&pixels, 44, 32)],
        [[255, 0, 0, 255], BACKGROUND_TEXEL]
    );
}

#[test]
fn ingested_view_frames_are_counted_once_per_render() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    harness.send_view(solid_frame(PixelSize::new(48, 48), [0, 0, 0, 255]));
    harness.send_view(solid_frame(PixelSize::new(48, 48), [0, 0, 0, 255]));
    let first = harness.render(Camera::default(), plain_grid(), &[]);
    let second = harness.render(Camera::default(), plain_grid(), &[]);
    assert_eq!((first.frames_received, second.frames_received), (2, 0));
}

#[test]
fn dropped_frame_events_are_counted() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    harness
        .compositor
        .handle_page_event(PageEvent::FrameDropped { page: PAGE })
        .unwrap();
    let stats = harness.render(Camera::default(), plain_grid(), &[]);
    assert_eq!(stats.frames_dropped_for_pool_pressure, 1);
}

#[test]
fn first_render_after_frame_reports_its_paint_wait_once() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    harness.send_view(solid_frame(PixelSize::new(48, 48), [0, 0, 0, 255]));
    let pages = [PageDraw {
        page: PAGE,
        rect: PAGE_RECT,
    }];
    let first = harness.render(Camera::default(), plain_grid(), &pages);
    let second = harness.render(Camera::default(), plain_grid(), &pages);
    assert_eq!(
        (
            first.max_paint_to_submit.is_some(),
            second.max_paint_to_submit.is_some()
        ),
        (true, false)
    );
}

#[test]
fn dirty_rect_update_leaves_clean_region_untouched() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let size = PixelSize::new(48, 48);
    harness.send_view(solid_frame(size, [0, 0, 100, 255]));
    let mut update = solid_frame(size, [0, 0, 250, 255]);
    update.dirty = vec![PixelRect::new(0, 0, 24, 48)];
    harness.send_view(update);
    let pixels = harness.render_page();
    // Left half of the page (screen x 8..32) updated, right half kept.
    assert_eq!(
        [pixel(&pixels, 20, 32), pixel(&pixels, 44, 32)],
        [[250, 0, 0, 255], [100, 0, 0, 255]]
    );
}

#[test]
fn popup_frame_draws_over_view_until_hidden() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    harness.send_view(solid_frame(PixelSize::new(48, 48), [0, 0, 100, 255]));
    let popup_rect = PixelRect::new(16, 16, 16, 16);
    harness
        .compositor
        .handle_page_event(frame_event(
            FrameLayer::Popup { rect: popup_rect },
            solid_frame(PixelSize::new(16, 16), [0, 200, 0, 255]),
        ))
        .unwrap();
    let shown = pixel(&harness.render_page(), 32, 32);
    harness
        .compositor
        .handle_page_event(PageEvent::PopupVisibility {
            page: PAGE,
            visible: false,
        })
        .unwrap();
    let hidden = pixel(&harness.render_page(), 32, 32);
    assert_eq!([shown, hidden], [[0, 200, 0, 255], [100, 0, 0, 255]]);
}

#[test]
fn removed_page_is_no_longer_drawn() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    harness.send_view(solid_frame(PixelSize::new(48, 48), [0, 0, 100, 255]));
    harness.compositor.remove_page(PAGE);
    let pixels = harness.render_page();
    assert_eq!(pixel(&pixels, 32, 32), BACKGROUND_TEXEL);
}

#[test]
fn malformed_cpu_frame_is_rejected_with_import_error() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let mut frame = solid_frame(PixelSize::new(48, 48), [0, 0, 0, 255]);
    frame.bgra.truncate(16);
    let result = harness
        .compositor
        .handle_page_event(frame_event(FrameLayer::View, frame));
    assert!(matches!(
        result,
        Err(CompositorError::Import {
            source: FrameImportError::ShortBuffer { .. },
            ..
        })
    ));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn shared_frame_without_platform_import_is_released_immediately() {
    use std::cell::Cell;
    use std::ptr::NonNull;
    use std::rc::Rc;

    use specular_core::{NativeSurface, PixelFormat, SharedTexture};

    let Some(mut harness) = Harness::new() else {
        return;
    };
    let released = Rc::new(Cell::new(false));
    let flag = Rc::clone(&released);
    let shared = SharedTexture::new(
        NativeSurface::IoSurface(NonNull::dangling()),
        PixelSize::new(4, 4),
        PixelFormat::Bgra8Unorm,
        move || flag.set(true),
    );
    let result = harness
        .compositor
        .handle_page_event(PageEvent::Frame(FrameEvent {
            page: PAGE,
            layer: FrameLayer::View,
            frame: PageFrame::GpuShared(shared),
            produced_at: Instant::now(),
        }));
    assert!(result.is_err() && released.get());
}
