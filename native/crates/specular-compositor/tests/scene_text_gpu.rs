//! Text through `Compositor::render_scene`, checked on an offscreen target.
//! Glyph shapes depend on the installed fonts, so these tests only ask where
//! the ink is. Each passes with a printed skip when there is no GPU adapter.

mod common;
mod scene_harness;

use glam::Vec2;
use scene_harness::{Harness, RED, RED_TEXEL, frame, ink, rect};
use specular_compositor::FrameView;
use specular_core::Camera;
use specular_scene::{Color, Item, Point, Rect, TextRun};

fn label(size: f32) -> TextRun {
    TextRun::new("MM", Point::new(4.0, 4.0), size, Color::BLACK)
}

#[test]
fn text_puts_ink_inside_its_box() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let pixels = harness.render(vec![Item::canvas(label(16.0))]);
    let (left, top, right, bottom) = ink(&pixels).expect("no glyph was drawn");
    assert!(
        left >= 3 && top >= 4 && right < 45 && bottom < 28,
        "{left} {top} {right} {bottom}"
    );
}

#[test]
fn text_under_the_size_floor_is_not_drawn() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // 16 units at zoom 0.1 is 1.6 px.
    let tiny = frame(Camera::new(Vec2::ZERO, 0.1));
    let (pixels, stats) = harness.render_frame(&tiny, vec![Item::canvas(label(16.0))]);
    assert_eq!((stats.text_runs_too_small, ink(&pixels)), (1, None));
}

#[test]
fn text_and_shapes_keep_their_order() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let cover = || Item::canvas(rect(0.0, 0.0, 64.0, 64.0, RED));
    let covered = harness.render(vec![Item::canvas(label(16.0)), cover()]);
    let on_top = harness.render(vec![cover(), Item::canvas(label(16.0))]);
    let all_red = |pixels: &[[u8; 4]]| pixels.iter().all(|&texel| texel == RED_TEXEL);
    assert!(all_red(&covered) && !all_red(&on_top));
}

#[test]
fn clipped_text_stays_inside_its_clip() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let item = Item::canvas(label(32.0)).clipped(Rect::new(0.0, 0.0, 20.0, 64.0));
    let (_, _, right, _) = ink(&harness.render(vec![item])).expect("no glyph was drawn");
    assert!(right < 20, "ink reaches column {right}");
}

#[test]
fn zooming_stretches_held_glyphs_to_where_sharp_ones_would_be() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let mut right_edge = |zoom: f32, zooming: bool| {
        let frame = FrameView {
            zooming,
            ..frame(Camera::new(Vec2::ZERO, zoom))
        };
        let (pixels, _) = harness.render_frame(&frame, vec![Item::canvas(label(16.0))]);
        ink(&pixels).expect("no glyph was drawn").2
    };
    let still = right_edge(1.0, false);
    // Within the hold band: the zoom-1 glyphs are stretched by 1.2.
    let held = right_edge(1.2, true);
    let settled = right_edge(1.2, false);
    assert!(
        held > still + 2 && held.abs_diff(settled) <= 2,
        "still {still}, held {held}, settled {settled}"
    );
}

#[test]
fn text_is_drawn_on_an_srgb_target_and_at_two_pixels_per_point() {
    let Some(mut harness) = Harness::with_format(wgpu::TextureFormat::Rgba8UnormSrgb) else {
        return;
    };
    let hidpi = FrameView {
        viewport: Vec2::splat(32.0),
        scale_factor: 2.0,
        ..frame(Camera::default())
    };
    let (pixels, stats) = harness.render_frame(&hidpi, vec![Item::screen(label(8.0))]);
    // An 8 point label at (4, 4) starts at physical pixel 8.
    let (left, top, ..) = ink(&pixels).expect("no glyph was drawn");
    assert!(
        left >= 7 && top >= 8 && stats.text_batches == 1,
        "{left} {top}"
    );
}
