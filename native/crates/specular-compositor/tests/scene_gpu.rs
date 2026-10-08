//! `Compositor::render_scene` checked pixel by pixel on an offscreen target:
//! shapes, paths, images, clips and z-order against pages. Each test passes
//! with a printed skip on machines with no GPU adapter.

mod common;
mod ink;
mod scene_harness;

use std::time::Instant;

use common::pixel;
use glam::Vec2;
use ink::{RED, RED_TEXEL, ink, rect};
use scene_harness::{BACKGROUND, Harness, PAGE, frame};
use specular_compositor::FrameView;
use specular_core::{Camera, CpuFrame, FrameEvent, FrameLayer, PageEvent, PageFrame, PixelSize};
use specular_doc::EntityId;
use specular_scene::{
    Color, Dash, EllipseDraw, ImageDraw, ImageId, Item, LineCap, PageDraw, PathDraw, PathStroke,
    Point, PolygonDraw, Rect, Stroke, StrokeAlign,
};

const GREEN: Color = Color::rgb(0, 255, 0);
const GREEN_TEXEL: [u8; 4] = [0, 255, 0, 255];

/// Paints the page solid green.
fn paint_page(harness: &mut Harness) {
    let size = PixelSize::new(32, 32);
    let frame = CpuFrame {
        size,
        stride: size.width * 4,
        bgra: [0, 255, 0, 255].repeat(size.area() as usize),
        dirty: Vec::new(),
    };
    let result = harness
        .compositor
        .handle_page_event(PageEvent::Frame(FrameEvent {
            page: PAGE,
            layer: FrameLayer::View,
            frame: PageFrame::Cpu(frame),
            produced_at: Instant::now(),
        }));
    assert!(result.is_ok(), "frame rejected: {result:?}");
}

fn page() -> PageDraw {
    PageDraw {
        page: EntityId::new("page"),
        rect: Rect::new(16.0, 16.0, 32.0, 32.0),
        corner_radius: 0.0,
    }
}

#[test]
fn a_rect_fills_its_pixels_and_no_others() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let pixels = harness.render(vec![Item::canvas(rect(16.0, 16.0, 32.0, 32.0, RED))]);
    assert_eq!(
        (pixel(&pixels, 32, 32), ink(&pixels)),
        (RED_TEXEL, Some((16, 16, 47, 47)))
    );
}

#[test]
fn an_inside_stroke_is_drawn_within_the_rect_edge() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let draw =
        rect(16.0, 16.0, 32.0, 32.0, RED).with_stroke(Stroke::new(GREEN, 2.0, StrokeAlign::Inside));
    let pixels = harness.render(vec![Item::canvas(draw)]);
    // Pixels 16 and 17 are the stroke, 18 is fill, 15 is outside.
    assert_eq!(
        [
            pixel(&pixels, 15, 32),
            pixel(&pixels, 17, 32),
            pixel(&pixels, 18, 32)
        ],
        [BACKGROUND, GREEN_TEXEL, RED_TEXEL]
    );
}

#[test]
fn an_ellipse_fills_its_middle_and_leaves_its_corners() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let draw = EllipseDraw::filled(Rect::new(8.0, 16.0, 48.0, 32.0), RED);
    let pixels = harness.render(vec![Item::canvas(draw)]);
    assert_eq!(
        [
            pixel(&pixels, 32, 32),
            pixel(&pixels, 10, 32),
            pixel(&pixels, 10, 18)
        ],
        [RED_TEXEL, RED_TEXEL, BACKGROUND]
    );
}

#[test]
fn a_page_between_two_items_keeps_all_three_in_order() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    paint_page(&mut harness);
    let items = vec![
        Item::canvas(rect(8.0, 8.0, 16.0, 16.0, RED)),
        Item::canvas(page()),
        Item::canvas(rect(40.0, 40.0, 16.0, 16.0, RED)),
    ];
    let (pixels, stats) = harness.render_frame(&frame(Camera::default()), items);
    // The first rect shows only outside the page; the second covers it.
    assert_eq!(
        (
            [
                pixel(&pixels, 10, 10),
                pixel(&pixels, 20, 20),
                pixel(&pixels, 44, 44)
            ],
            stats.batches
        ),
        ([RED_TEXEL, GREEN_TEXEL, RED_TEXEL], 3)
    );
}

#[test]
fn a_page_with_no_frame_is_counted_and_not_drawn() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let (pixels, stats) =
        harness.render_frame(&frame(Camera::default()), vec![Item::canvas(page())]);
    assert_eq!(
        (stats.render.pages_without_texture, ink(&pixels)),
        (1, None)
    );
}

#[test]
fn a_polygon_is_filled_inside_its_outline() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let diamond = PolygonDraw {
        points: vec![
            Point::new(32.0, 8.0),
            Point::new(56.0, 32.0),
            Point::new(32.0, 56.0),
            Point::new(8.0, 32.0),
        ],
        fill: Some(RED),
        stroke: None,
    };
    let pixels = harness.render(vec![Item::canvas(diamond)]);
    assert_eq!(
        [pixel(&pixels, 32, 32), pixel(&pixels, 12, 12)],
        [RED_TEXEL, BACKGROUND]
    );
}

#[test]
fn a_stroked_path_covers_its_width() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let stroke = PathStroke {
        cap: LineCap::Butt,
        ..PathStroke::new(RED, 4.0)
    };
    let line = PathDraw::polyline([Point::new(8.0, 32.0), Point::new(56.0, 32.0)], stroke);
    let pixels = harness.render(vec![Item::canvas(line)]);
    assert_eq!(ink(&pixels), Some((8, 30, 55, 33)));
}

#[test]
fn a_dashed_path_leaves_its_gaps_clear() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let stroke = PathStroke {
        cap: LineCap::Butt,
        ..PathStroke::new(RED, 4.0).dashed(Dash { on: 8.0, off: 8.0 })
    };
    let line = PathDraw::polyline([Point::new(8.0, 32.0), Point::new(56.0, 32.0)], stroke);
    let pixels = harness.render(vec![Item::canvas(line)]);
    // Dashes cover x 8..16 and 24..32; the gap between is 16..24.
    assert_eq!(
        [
            pixel(&pixels, 12, 32),
            pixel(&pixels, 20, 32),
            pixel(&pixels, 28, 32)
        ],
        [RED_TEXEL, BACKGROUND, RED_TEXEL]
    );
}

#[test]
fn a_clip_cuts_an_item_to_its_rect() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let item =
        Item::canvas(rect(8.0, 8.0, 48.0, 48.0, RED)).clipped(Rect::new(16.0, 24.0, 16.0, 8.0));
    let pixels = harness.render(vec![item]);
    assert_eq!(ink(&pixels), Some((16, 24, 31, 31)));
}

#[test]
fn opacity_blends_an_item_with_what_is_under_it() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let white = Color::rgb(255, 255, 255);
    let item = Item::canvas(rect(16.0, 16.0, 32.0, 32.0, white)).with_opacity(0.5);
    let [r, g, b, _] = pixel(&harness.render(vec![item]), 32, 32);
    // Half white over blue, blended in the target's encoding.
    assert!(r.abs_diff(128) <= 1 && g.abs_diff(128) <= 1 && b == 255);
}

#[test]
fn screen_items_ignore_the_camera_and_canvas_items_follow_it() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let items = vec![
        Item::canvas(rect(8.0, 8.0, 8.0, 8.0, RED)),
        Item::screen(rect(40.0, 40.0, 8.0, 8.0, GREEN)),
    ];
    // Zoom 2 and a pan of 4 put the canvas rect at 20..36.
    let (pixels, _) = harness.render_frame(&frame(Camera::new(Vec2::splat(4.0), 2.0)), items);
    assert_eq!(
        [
            pixel(&pixels, 28, 28),
            pixel(&pixels, 12, 12),
            pixel(&pixels, 44, 44)
        ],
        [RED_TEXEL, BACKGROUND, GREEN_TEXEL]
    );
}

#[test]
fn an_uploaded_image_is_drawn_and_can_be_cropped() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // Red on the left half, green on the right, two texels each so a sample
    // inside either half is not filtered with the other.
    let texels = [
        [255, 0, 0, 255],
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 255, 0, 255],
    ]
    .concat();
    harness
        .compositor
        .set_image(ImageId(1), PixelSize::new(4, 1), &texels)
        .unwrap();
    let whole = ImageDraw::new(ImageId(1), Rect::new(16.0, 16.0, 32.0, 32.0));
    let right_half = ImageDraw {
        source: Rect::new(0.5, 0.0, 0.5, 1.0),
        ..whole
    };
    let whole = harness.render(vec![Item::canvas(whole)]);
    let cropped = harness.render(vec![Item::canvas(right_half)]);
    assert_eq!(
        [
            pixel(&whole, 18, 32),
            pixel(&whole, 46, 32),
            pixel(&cropped, 32, 32)
        ],
        [RED_TEXEL, GREEN_TEXEL, GREEN_TEXEL]
    );
}

#[test]
fn an_image_drawn_small_is_sampled_from_its_mip_levels() {
    // Two white columns then six black, across 64 texels. Drawn at an eighth
    // of its size, every pixel covers one period and its centre lands on
    // black, so the full-size level alone would read black. Three levels
    // down each texel is the mean: a quarter white.
    const SIDE: u32 = 64;
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let texels: Vec<u8> = (0..SIDE * SIDE)
        .flat_map(|index| {
            let value = if index % SIDE % 8 < 2 { 255 } else { 0 };
            [value, value, value, 255]
        })
        .collect();
    harness
        .compositor
        .set_image(ImageId(1), PixelSize::new(SIDE, SIDE), &texels)
        .unwrap();
    let small = ImageDraw::new(ImageId(1), Rect::new(16.0, 16.0, 8.0, 8.0));
    let pixels = harness.render(vec![Item::canvas(small)]);
    for (x, y) in [(17, 17), (20, 20), (22, 19)] {
        let [r, g, b, a] = pixel(&pixels, x, y);
        assert!(
            r.abs_diff(64) <= 6 && r == g && g == b && a == 255,
            "pixel ({x}, {y}) is {:?}",
            [r, g, b, a]
        );
    }
}

#[test]
fn a_hidpi_frame_draws_and_clips_in_physical_pixels() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // 32 logical pixels across a 64 pixel target.
    let hidpi = FrameView {
        viewport: Vec2::splat(32.0),
        scale_factor: 2.0,
        ..frame(Camera::default())
    };
    let item =
        Item::canvas(rect(8.0, 8.0, 16.0, 16.0, RED)).clipped(Rect::new(8.0, 8.0, 16.0, 8.0));
    let (pixels, _) = harness.render_frame(&hidpi, vec![item]);
    assert_eq!(ink(&pixels), Some((16, 16, 47, 31)));
}

#[test]
fn an_srgb_target_shows_the_same_colours() {
    // The same mid grey on a target the shader encodes for and on one the
    // hardware encodes for.
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ] {
        let Some(mut harness) = Harness::with_format(format) else {
            return;
        };
        let grey = Color::rgb(128, 128, 128);
        let pixels = harness.render(vec![Item::canvas(rect(16.0, 16.0, 32.0, 32.0, grey))]);
        let [r, g, b, _] = pixel(&pixels, 32, 32);
        assert!(
            r.abs_diff(128) <= 1 && g.abs_diff(128) <= 1 && b.abs_diff(128) <= 1,
            "{format:?} shows {:?}",
            [r, g, b]
        );
    }
}
