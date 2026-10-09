//! Frame ingestion checked on offscreen renders, pixel by pixel: CPU frames,
//! dirty rects, popups, and the counters a render reports. Each test passes
//! with a printed skip on machines with no GPU adapter (the Linux CI runner).

mod common;
mod scene_harness;

use std::time::Instant;

use common::pixel;
use glam::Vec2;
use scene_harness::{BACKGROUND as BACKGROUND_TEXEL, Harness, PAGE, frame};
use specular_compositor::{CompositorError, DotGrid, FrameImportError, RenderStats};
use specular_core::{
    Camera, CpuFrame, CssSize, FrameEvent, FrameLayer, PageEvent, PageFrame, PixelRect, PixelSize,
};
use specular_doc::EntityId;
use specular_scene::{Item, PageDraw, Rect};

/// Corner radius the page is drawn with, in canvas units.
const CORNER_RADIUS: f32 = 8.0;

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
        viewport: CssSize::default(),
        frame: PageFrame::Cpu(frame),
        produced_at: Instant::now(),
    })
}

/// The page at canvas (8, 8), 48 units square.
fn page() -> Vec<Item> {
    vec![Item::canvas(PageDraw {
        page: EntityId::new("page"),
        rect: Rect::new(8.0, 8.0, 48.0, 48.0),
        viewport: CssSize::default(),
        corner_radius: CORNER_RADIUS,
    })]
}

/// Renders `items` and returns the page and frame counters.
fn stats(harness: &mut Harness, items: Vec<Item>) -> RenderStats {
    harness
        .render_frame(&frame(Camera::default()), items)
        .1
        .render
}

fn send_view(harness: &mut Harness, frame: CpuFrame) {
    let result = harness
        .compositor
        .handle_page_event(frame_event(FrameLayer::View, frame));
    assert!(result.is_ok(), "frame rejected: {result:?}");
}

#[test]
fn grid_draws_a_dot_at_the_world_origin() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // Pan puts the world origin on pixel (10, 10)'s centre.
    let view = frame(Camera::new(Vec2::splat(10.5), 1.0));
    let view = specular_compositor::FrameView {
        grid: DotGrid {
            spacing: 20.0,
            dot: [1.0, 0.0, 0.0, 1.0],
            ..view.grid
        },
        ..view
    };
    let (pixels, _) = harness.render_frame(&view, Vec::new());
    assert_eq!(
        [pixel(&pixels, 10, 10), pixel(&pixels, 20, 10)],
        [[255, 0, 0, 255], BACKGROUND_TEXEL]
    );
}

#[test]
fn cpu_frame_is_drawn_inside_page_rect() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    send_view(
        &mut harness,
        solid_frame(PixelSize::new(48, 48), [10, 20, 200, 255]),
    );
    let pixels = harness.render(page());
    assert_eq!(pixel(&pixels, 32, 32), [200, 20, 10, 255]);
}

#[test]
fn rounded_corner_reveals_background() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    send_view(
        &mut harness,
        solid_frame(PixelSize::new(48, 48), [10, 20, 200, 255]),
    );
    let pixels = harness.render(page());
    assert_eq!(pixel(&pixels, 8, 8), BACKGROUND_TEXEL);
}

#[test]
fn ingested_view_frames_are_counted_once_per_render() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    send_view(
        &mut harness,
        solid_frame(PixelSize::new(48, 48), [0, 0, 0, 255]),
    );
    send_view(
        &mut harness,
        solid_frame(PixelSize::new(48, 48), [0, 0, 0, 255]),
    );
    let first = stats(&mut harness, Vec::new());
    let second = stats(&mut harness, Vec::new());
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
    let stats = stats(&mut harness, Vec::new());
    assert_eq!(stats.frames_dropped_for_pool_pressure, 1);
}

#[test]
fn first_render_after_frame_reports_its_paint_wait_once() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    send_view(
        &mut harness,
        solid_frame(PixelSize::new(48, 48), [0, 0, 0, 255]),
    );
    let first = stats(&mut harness, page());
    let second = stats(&mut harness, page());
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
    send_view(&mut harness, solid_frame(size, [0, 0, 100, 255]));
    let mut update = solid_frame(size, [0, 0, 250, 255]);
    update.dirty = vec![PixelRect::new(0, 0, 24, 48)];
    send_view(&mut harness, update);
    let pixels = harness.render(page());
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
    send_view(
        &mut harness,
        solid_frame(PixelSize::new(48, 48), [0, 0, 100, 255]),
    );
    let popup_rect = PixelRect::new(16, 16, 16, 16);
    harness
        .compositor
        .handle_page_event(frame_event(
            FrameLayer::Popup { rect: popup_rect },
            solid_frame(PixelSize::new(16, 16), [0, 200, 0, 255]),
        ))
        .unwrap();
    let shown = pixel(&harness.render(page()), 32, 32);
    harness
        .compositor
        .handle_page_event(PageEvent::PopupVisibility {
            page: PAGE,
            visible: false,
        })
        .unwrap();
    let hidden = pixel(&harness.render(page()), 32, 32);
    assert_eq!([shown, hidden], [[0, 200, 0, 255], [100, 0, 0, 255]]);
}

#[test]
fn removed_page_is_no_longer_drawn() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    send_view(
        &mut harness,
        solid_frame(PixelSize::new(48, 48), [0, 0, 100, 255]),
    );
    harness.compositor.remove_page(PAGE);
    let pixels = harness.render(page());
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
            viewport: CssSize::default(),
            frame: PageFrame::GpuShared(shared),
            produced_at: Instant::now(),
        }));
    assert!(result.is_err() && released.get());
}
