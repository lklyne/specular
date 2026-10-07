//! The editor's caret against the drawn glyphs: text is edited in an `App`
//! whose measure is the compositor's own, `view` builds the scene, and the
//! render is read back to see which glyphs the caret landed between. Each
//! passes with a printed skip when there is no GPU adapter.

mod common;
mod scene_harness;

use std::ops::RangeInclusive;
use std::sync::Arc;

use common::{TARGET_SIZE, pixel};
use glam::Vec2;
use scene_harness::{BACKGROUND, Harness, frame};
use specular_core::Camera;
use specular_doc::{Entity, Kind, Rect, Text, TextStyle, WidthMode};
use specular_interact::Key;
use specular_scene::view_without_chrome;
use specular_testkit::{TestApp, document};

/// The runs of neighbouring columns `has` accepts, left to right.
fn column_runs(has: impl Fn(u32) -> bool) -> Vec<RangeInclusive<u32>> {
    let mut runs: Vec<RangeInclusive<u32>> = Vec::new();
    for x in (0..TARGET_SIZE).filter(|&x| has(x)) {
        match runs.last_mut() {
            Some(run) if *run.end() + 1 == x => *run = *run.start()..=x,
            _ => runs.push(x..=x),
        }
    }
    runs
}

/// The texels of column `x`, top to bottom.
fn column(pixels: &[[u8; 4]], x: u32) -> impl Iterator<Item = [u8; 4]> + '_ {
    (0..TARGET_SIZE).map(move |y| pixel(pixels, x, y))
}

/// What one caret position looks like: the columns each glyph inked, and
/// the columns the caret added.
struct Shot {
    glyphs: Vec<RangeInclusive<u32>>,
    caret: RangeInclusive<u32>,
}

/// Plain text reading `text` in `size` px type, edited at `zoom` with the
/// caret after `offset` characters. Rendered with the caret shown and with
/// it blinked off, and the difference is the caret.
fn shoot(harness: &mut Harness, text: &str, size: f64, zoom: f32, offset: usize) -> Shot {
    let entity = Entity::new(
        "t",
        Rect::new(4.0, 8.0, 10.0, 10.0),
        Kind::Text(Text {
            text: text.to_owned(),
            style: Some(TextStyle::Plain),
            width_mode: Some(WidthMode::Auto),
            size: Some(size),
            ..Text::default()
        }),
    );
    let mut app = TestApp::empty();
    app.measure_with(Arc::new(harness.compositor.text_measure()));
    app.open(document([entity])).zoom(zoom);
    // The middle of the text, clear of the handles a first click brings up.
    app.double_click(Vec2::new(36.0, 20.0) * zoom)
        .chord(specular_testkit::CMD, Key::ArrowUp);
    for _ in 0..offset {
        app.key(Key::ArrowRight);
    }
    assert_eq!(app.caret(), (offset, offset), "the text is ASCII");
    let camera = app.session().camera;
    let mut render = |app: &TestApp| {
        let scene = view_without_chrome(app.app(), Vec2::splat(TARGET_SIZE as f32));
        if camera == Camera::default() {
            harness.render(scene.items)
        } else {
            harness.render_frame(&frame(camera), scene.items).0
        }
    };
    let shown = render(&app);
    app.tick(500);
    assert!(!app.app().caret_visible());
    let hidden = render(&app);
    let glyphs = column_runs(|x| column(&hidden, x).any(|texel| texel != BACKGROUND));
    let caret = column_runs(|x| column(&shown, x).ne(column(&hidden, x)));
    assert_eq!(caret.len(), 1, "one caret: {caret:?} beside {glyphs:?}");
    Shot {
        glyphs,
        caret: caret[0].clone(),
    }
}

/// Whether the caret sits after glyph `left` and before glyph `right`,
/// either of which may be missing at an end of the line. A pixel of overlap
/// with a glyph's antialiased edge is allowed.
fn sits_between(shot: &Shot, left: Option<usize>, right: Option<usize>) -> bool {
    let after = left.is_none_or(|at| *shot.caret.start() + 1 >= *shot.glyphs[at].end());
    let before = right.is_none_or(|at| *shot.caret.end() <= *shot.glyphs[at].start() + 1);
    after && before
}

#[test]
fn the_caret_sits_between_the_glyphs_its_offset_is_between() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // Three letters with spaces between: glyph n is at offset 2n.
    let cases = [
        (0, None, Some(0)),
        (1, Some(0), Some(1)),
        (2, Some(0), Some(1)),
        (3, Some(1), Some(2)),
        (4, Some(1), Some(2)),
        (5, Some(2), None),
    ];
    for (offset, left, right) in cases {
        let shot = shoot(&mut harness, "M M M", 16.0, 1.0, offset);
        assert_eq!(shot.glyphs.len(), 3, "{:?}", shot.glyphs);
        assert!(
            sits_between(&shot, left, right),
            "offset {offset}: caret {:?} among glyphs {:?}",
            shot.caret,
            shot.glyphs
        );
    }
}

#[test]
fn a_caret_before_a_space_hugs_the_glyph_and_one_after_it_hugs_the_next() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let before = shoot(&mut harness, "M M M", 16.0, 1.0, 1);
    let after = shoot(&mut harness, "M M M", 16.0, 1.0, 2);
    let (first, second) = (*before.glyphs[0].end(), *before.glyphs[1].start());
    assert!(
        before.caret.start().abs_diff(first) <= 2 && after.caret.end().abs_diff(second) <= 2,
        "carets {:?} and {:?} around the gap {first}..{second}",
        before.caret,
        after.caret
    );
}

#[test]
fn the_caret_stays_on_its_glyphs_when_the_camera_zooms() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    for zoom in [0.5, 2.0] {
        let cases = [
            (0, None, Some(0)),
            (1, Some(0), Some(1)),
            (3, Some(1), None),
        ];
        for (offset, left, right) in cases {
            let shot = shoot(
                &mut harness,
                "M M",
                12.0 / f64::from(zoom) * 1.5,
                zoom,
                offset,
            );
            assert_eq!(shot.glyphs.len(), 2, "{:?}", shot.glyphs);
            assert!(
                sits_between(&shot, left, right),
                "zoom {zoom}, offset {offset}: caret {:?} among glyphs {:?}",
                shot.caret,
                shot.glyphs
            );
        }
    }
}
