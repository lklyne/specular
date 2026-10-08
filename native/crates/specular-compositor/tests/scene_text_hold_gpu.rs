//! Text keeps the glyph quads it laid out while the camera pans or a held
//! zoom runs, and draws them where a fresh layout would. Checked on an
//! offscreen target; each test passes with a printed skip when there is no
//! GPU adapter.

mod common;
mod ink;
mod scene_harness;

use glam::Vec2;
use ink::{RED, RED_TEXEL, ink, rect};
use scene_harness::{Harness, frame};
use specular_compositor::{FrameView, TextCounts};
use specular_core::Camera;
use specular_scene::{Color, Item, Point, Rect, TextRun};

fn label(text: &str) -> Vec<Item> {
    let run = TextRun::new(text, Point::new(8.0, 8.0), 16.0, Color::BLACK);
    vec![Item::canvas(run)]
}

fn at(pan: Vec2) -> FrameView {
    frame(Camera::new(pan, 1.0))
}

/// What a compositor that has drawn nothing yet draws for `frame`.
fn fresh(frame: &FrameView, items: Vec<Item>) -> Option<Vec<[u8; 4]>> {
    Some(Harness::new()?.render_frame(frame, items).0)
}

fn laid_out_and_reused(counts: TextCounts) -> (u32, u32) {
    (counts.laid_out, counts.reused)
}

#[test]
fn a_still_frame_lays_no_text_out_again() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let (first, stats) = harness.render_frame(&at(Vec2::ZERO), label("MM"));
    assert_eq!(laid_out_and_reused(stats.text), (1, 0));
    let (second, stats) = harness.render_frame(&at(Vec2::ZERO), label("MM"));
    assert_eq!(laid_out_and_reused(stats.text), (0, 1));
    assert!(!stats.text.settling && stats.glyphs == 2);
    assert_eq!(first, second);
}

#[test]
fn a_pan_draws_the_layout_it_has_where_a_fresh_one_would_be() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    harness.render_frame(&at(Vec2::ZERO), label("MM"));
    for pan in [Vec2::new(9.0, -5.0), Vec2::new(-6.0, 21.0)] {
        let (panned, stats) = harness.render_frame(&at(pan), label("MM"));
        assert_eq!(laid_out_and_reused(stats.text), (0, 1), "pan {pan}");
        assert!(!stats.text.settling);
        assert_eq!(Some(panned), fresh(&at(pan), label("MM")), "pan {pan}");
    }
}

#[test]
fn a_pan_brings_in_glyphs_that_were_off_the_target_when_laid_out() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // The run starts left of the target: only its tail shows at first.
    let long = || {
        let run = TextRun::new("MMMMMMMMMM", Point::new(-90.0, 8.0), 16.0, Color::BLACK);
        vec![Item::canvas(run)]
    };
    harness.render_frame(&at(Vec2::ZERO), long());
    let pan = Vec2::new(60.0, 0.0);
    let (panned, stats) = harness.render_frame(&at(pan), long());
    assert_eq!(laid_out_and_reused(stats.text), (0, 1));
    assert_eq!(Some(panned), fresh(&at(pan), long()));
}

#[test]
fn changed_text_is_laid_out_again() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    harness.render_frame(&at(Vec2::ZERO), label("MM"));
    let (_, stats) = harness.render_frame(&at(Vec2::ZERO), label("MW"));
    assert_eq!(laid_out_and_reused(stats.text), (1, 0));
}

#[test]
fn text_gone_from_the_scene_while_in_sight_is_not_drawn_from_the_kept_layout() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let run = |text: &str, y: f32| TextRun::new(text, Point::new(8.0, y), 16.0, Color::BLACK);
    // A popup's label beside a readout that stays, in screen space as the
    // panels draw them, and two notes on the canvas.
    let spaces: [fn(TextRun) -> Item; 2] = [Item::screen, Item::canvas];
    for item in spaces {
        let both = || vec![item(run("MM", 8.0)), item(run("WW", 36.0))];
        let one = || vec![item(run("MM", 8.0))];
        harness.render_frame(&at(Vec2::ZERO), both());
        let (left, stats) = harness.render_frame(&at(Vec2::ZERO), one());
        assert_eq!(laid_out_and_reused(stats.text), (1, 0));
        assert_eq!(Some(left), fresh(&at(Vec2::ZERO), one()));
    }
}

#[test]
fn a_pan_by_part_of_a_pixel_is_put_right_by_the_next_frame_at_rest() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    harness.render_frame(&at(Vec2::ZERO), label("MM"));
    let pan = Vec2::new(10.4, 0.0);
    let (_, moving) = harness.render_frame(&at(pan), label("MM"));
    assert_eq!(laid_out_and_reused(moving.text), (0, 1));
    assert!(moving.text.settling);
    let (rested, rest) = harness.render_frame(&at(pan), label("MM"));
    assert_eq!(laid_out_and_reused(rest.text), (1, 0));
    assert!(!rest.text.settling);
    assert_eq!(Some(rested), fresh(&at(pan), label("MM")));
    // And then it is kept.
    let (_, kept) = harness.render_frame(&at(pan), label("MM"));
    assert_eq!(laid_out_and_reused(kept.text), (0, 1));
}

#[test]
fn a_held_zoom_keeps_the_layout_and_the_frame_after_it_does_not() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let zoomed = |zoom: f32, zooming: bool| FrameView {
        zooming,
        ..frame(Camera::new(Vec2::ZERO, zoom))
    };
    harness.render_frame(&zoomed(1.0, false), label("MM"));
    for zoom in [1.05, 1.1, 1.2] {
        let (_, stats) = harness.render_frame(&zoomed(zoom, true), label("MM"));
        assert_eq!(laid_out_and_reused(stats.text), (0, 1), "zoom {zoom}");
    }
    let (_, settled) = harness.render_frame(&zoomed(1.2, false), label("MM"));
    assert_eq!(laid_out_and_reused(settled.text), (1, 0));
}

#[test]
fn a_clip_moves_with_the_text_it_cuts() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let clipped = || {
        let run = TextRun::new("MMMM", Point::new(4.0, 8.0), 24.0, Color::BLACK);
        vec![Item::canvas(run).clipped(Rect::new(0.0, 0.0, 30.0, 64.0))]
    };
    harness.render_frame(&at(Vec2::ZERO), clipped());
    let pan = Vec2::new(20.0, 0.0);
    let (panned, stats) = harness.render_frame(&at(pan), clipped());
    assert_eq!(laid_out_and_reused(stats.text), (0, 1));
    let (_, _, right, _) = ink(&panned).expect("no glyph was drawn");
    assert!(right < 50, "ink reaches column {right}");
    assert_eq!(Some(panned), fresh(&at(pan), clipped()));
}

#[test]
fn kept_text_stays_under_what_is_drawn_over_it() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let covered = || {
        let mut items = label("MM");
        items.push(Item::canvas(rect(0.0, 0.0, 64.0, 64.0, RED)));
        items
    };
    harness.render(covered());
    // The second frame draws the text from the layout the first made.
    let pixels = harness.render(covered());
    assert!(pixels.iter().all(|&texel| texel == RED_TEXEL));
}
