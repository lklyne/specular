//! Shadows, the multiply blend and the page band's fade, checked pixel by
//! pixel on an offscreen target. Each test passes with a printed skip on
//! machines with no GPU adapter.

mod common;
#[expect(
    dead_code,
    reason = "shared with the other GPU tests, which use the rest"
)]
mod ink;
mod scene_harness;

use common::pixel;
use ink::rect;
use scene_harness::{BACKGROUND, Harness};
use specular_scene::{Blend, Color, Item, PageBand, Point, PolygonDraw, Rect, ShadowDraw};

/// A black shadow of a 16 px square in the middle of the target, over the
/// blue background: the blue channel is what the shadow lets through.
fn shadow_blue(harness: &mut Harness, blur: f32, opacity: u8) -> impl Fn(u32) -> u8 {
    let shadow = ShadowDraw {
        rect: Rect::new(24.0, 24.0, 16.0, 16.0),
        corner_radius: 0.0,
        blur,
        color: Color::rgba(0, 0, 0, opacity),
    };
    let pixels = harness.render(vec![Item::canvas(shadow)]);
    move |x| pixel(&pixels, x, 32)[2]
}

#[test]
fn a_shadow_is_half_gone_at_its_casters_edge_and_gone_well_outside_it() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let blue = shadow_blue(&mut harness, 4.0, 255);
    // Dense in the middle, half at the edge (x = 40), clear by x = 50.
    assert!(blue(32) < 8, "middle {}", blue(32));
    let edge = u16::midpoint(u16::from(blue(39)), u16::from(blue(40)));
    assert!((112..=143).contains(&edge), "edge {edge}");
    assert_eq!(blue(50), BACKGROUND[2]);
    // It only ever thins on the way out.
    let fade: Vec<u8> = (32..52).map(&blue).collect();
    let thins = |pair: &[u8]| pair[0] <= pair[1].saturating_add(1);
    assert!(fade.windows(2).all(thins), "{fade:?}");
    assert!(blue(42) < 240, "still there half a blur out: {}", blue(42));
}

#[test]
fn a_shadows_colour_alpha_is_its_densest() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // 20 of 255, as a sticky's shadow is.
    let blue = shadow_blue(&mut harness, 2.0, 20);
    assert!((233..=237).contains(&blue(32)), "middle {}", blue(32));
}

#[test]
fn a_shadow_with_no_blur_is_its_casters_shape() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let blue = shadow_blue(&mut harness, 0.0, 255);
    // The edge keeps a pixel of softness, so it does not alias.
    assert_eq!([blue(22), blue(26), blue(37), blue(41)], [255, 0, 0, 255]);
    assert!(blue(24) < 64 && blue(23) > 191, "{} {}", blue(24), blue(23));
}

fn band(y: f32, fill: Color) -> PolygonDraw {
    PolygonDraw {
        points: vec![
            Point::new(8.0, y),
            Point::new(56.0, y),
            Point::new(56.0, y + 16.0),
            Point::new(8.0, y + 16.0),
        ],
        fill: Some(fill),
        stroke: None,
    }
}

#[test]
fn a_multiplied_fill_tints_white_and_leaves_black_black() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let yellow = Color::rgb(255, 255, 0);
    let pixels = harness.render(vec![
        Item::canvas(rect(8.0, 8.0, 24.0, 48.0, Color::WHITE)),
        Item::canvas(rect(32.0, 8.0, 24.0, 48.0, Color::BLACK)),
        // Painted over, the lower band would turn both yellow.
        Item::canvas(band(16.0, yellow)).with_blend(Blend::Multiply),
        Item::canvas(band(40.0, yellow)),
    ]);
    assert_eq!(
        [
            pixel(&pixels, 20, 24),
            pixel(&pixels, 44, 24),
            pixel(&pixels, 20, 48),
            pixel(&pixels, 44, 48),
            pixel(&pixels, 20, 10),
        ],
        [
            [255, 255, 0, 255],
            [0, 0, 0, 255],
            [255, 255, 0, 255],
            [255, 255, 0, 255],
            [255, 255, 255, 255],
        ]
    );
}

#[test]
fn a_multiplied_fill_at_part_alpha_tints_part_way() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // On mid grey, half-covering yellow multiplied keeps red and green and
    // halves blue; laid over as ordinary paint it would lighten red and green.
    let pixels = harness.render(vec![
        Item::canvas(rect(8.0, 8.0, 48.0, 48.0, Color::rgb(200, 200, 200))),
        Item::canvas(band(16.0, Color::rgba(255, 255, 0, 128))).with_blend(Blend::Multiply),
    ]);
    let [r, g, b, a] = pixel(&pixels, 20, 24);
    assert_eq!((r, g, a), (200, 200, 255));
    assert!((95..=105).contains(&b), "blue {b}");
}

/// The page of the band the tests draw through: columns 16 to 48 and rows
/// 24 to 40, with the fade reaching 16 rows past it each way.
const BAND: PageBand = PageBand {
    page: Rect::new(16.0, 24.0, 32.0, 16.0),
    reach: 16.0,
};

#[test]
fn what_shows_through_a_page_band_is_whole_over_the_page_and_thins_to_nothing_past_it() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // A white column the whole height of the target, as a shape and as a
    // mesh.
    let outline = PolygonDraw {
        points: vec![
            Point::new(8.0, 0.0),
            Point::new(56.0, 0.0),
            Point::new(56.0, 64.0),
            Point::new(8.0, 64.0),
        ],
        fill: Some(Color::WHITE),
        stroke: None,
    };
    let columns = [
        (
            "shape",
            Item::canvas(rect(8.0, 0.0, 48.0, 64.0, Color::WHITE)),
        ),
        ("mesh", Item::canvas(outline)),
    ];
    for (kind, column) in columns {
        let mut items = Vec::new();
        BAND.through(column, Rect::new(8.0, 0.0, 48.0, 64.0), &mut items);
        let pixels = harness.render(items);
        let red = |y| pixel(&pixels, 32, y)[0];
        // Whole over the page, and gone past the reach of the fade.
        assert_eq!([red(25), red(32), red(38)], [255, 255, 255], "{kind}");
        assert_eq!([red(2), red(6), red(58), red(62)], [0, 0, 0, 0], "{kind}");
        // About half way gone half way out, above and below.
        for half in [16, 47] {
            assert!((100..=155).contains(&red(half)), "{kind} {}", red(half));
        }
        // It only ever thins on the way out, and never by a visible jump:
        // a step is a sixteenth of the way.
        let up: Vec<u8> = (8..=24).rev().map(red).collect();
        let down: Vec<u8> = (39..=55).map(red).collect();
        for fade in [up, down] {
            let thins = |pair: &[u8]| pair[1] <= pair[0] && pair[0] - pair[1] <= 24;
            assert!(fade.windows(2).all(thins), "{kind} {fade:?}");
            assert!(fade[fade.len() - 1] < 24, "{kind} {fade:?}");
        }
        // The page's sides cut hard, over the page and over the fade.
        for row in [32, 16] {
            let red = |x| pixel(&pixels, x, row)[0];
            assert_eq!([red(12), red(15), red(50)], [0, 0, 0], "{kind} row {row}");
            assert!(red(16) > 100 && red(47) > 100, "{kind} row {row}");
        }
    }
}
