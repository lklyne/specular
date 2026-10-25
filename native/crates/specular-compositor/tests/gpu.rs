//! What `Compositor::render_scene` puts on an offscreen target, read back
//! pixel by pixel: one test a rendering rule a scene snapshot cannot see (a
//! colour, coverage, a blend), and the pins on work a still frame skips.
//! Glyph shapes depend on the installed fonts, so text tests only ask where
//! the ink is and what colour. Each passes with a printed skip on machines
//! with no GPU adapter (the Linux CI runner).

mod common;
mod ink;
mod scene_harness;

use std::time::Instant;

use common::{TARGET_SIZE, pixel};
use glam::Vec2;
use ink::{RED, RED_TEXEL, ink, rect};
use scene_harness::{BACKGROUND, Harness, PAGE, frame};
use specular_compositor::{FrameView, TextCounts};
use specular_core::{
    Camera, CpuFrame, CssSize, FrameEvent, FrameLayer, PageEvent, PageFrame, PixelSize,
};
use specular_doc::EntityId;
use specular_scene::{
    Blend, Color, ImageDraw, ImageId, Item, PageDraw, Point, PolygonDraw, Rect, ShadowDraw,
    SpanStyle, TextRun, TextSpan,
};

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
fn text_keeps_its_colour_on_an_srgb_target() {
    let Some(mut harness) = Harness::with_format(wgpu::TextureFormat::Rgba8UnormSrgb) else {
        return;
    };
    // Inside a large glyph the ink is the text colour alone, whatever the
    // blend: a mid grey reads as that grey, not a lighter one.
    let grey = Color::rgb(128, 128, 128);
    let text = TextRun::new("MM", Point::new(4.0, 4.0), 40.0, grey);
    let pixels = harness.render(vec![Item::canvas(text)]);
    let solid: Vec<u8> = pixels
        .iter()
        .filter(|[r, g, b, _]| r == g && g == b)
        .map(|texel| texel[0])
        .collect();
    assert!(
        !solid.is_empty() && solid.iter().all(|value| value.abs_diff(128) <= 2),
        "grey ink reads {:?}",
        solid.iter().max()
    );
}

/// `(left, top, right, bottom)` of the pixels `is` accepts.
fn bounds_of(pixels: &[[u8; 4]], is: impl Fn([u8; 4]) -> bool) -> Option<(u32, u32, u32, u32)> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for y in 0..TARGET_SIZE {
        for x in 0..TARGET_SIZE {
            if is(pixel(pixels, x, y)) {
                let (left, top, right, bottom) = bounds.unwrap_or((x, y, x, y));
                bounds = Some((left.min(x), top.min(y), right.max(x), bottom.max(y)));
            }
        }
    }
    bounds
}

fn is_red([r, g, b, _]: [u8; 4]) -> bool {
    r > 200 && g < 60 && b < 60
}

fn is_black([r, g, b, _]: [u8; 4]) -> bool {
    r < 60 && g < 60 && b < 60
}

fn run(text: &str, spans: Vec<(std::ops::Range<usize>, SpanStyle)>) -> TextRun {
    TextRun {
        spans: spans
            .into_iter()
            .map(|(range, style)| TextSpan { range, style })
            .collect(),
        ..TextRun::new(text, Point::new(4.0, 4.0), 12.0, Color::BLACK)
    }
}

fn render(harness: &mut Harness, run: TextRun) -> Vec<[u8; 4]> {
    harness.render(vec![Item::canvas(run)])
}

#[test]
fn a_coloured_span_is_drawn_in_its_colour_beside_the_rest() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let red = SpanStyle {
        color: Some(RED),
        ..SpanStyle::default()
    };
    let pixels = render(&mut harness, run("MM MM", vec![(3..5, red)]));
    let black = bounds_of(&pixels, is_black).expect("no black glyph was drawn");
    let red = bounds_of(&pixels, is_red).expect("no red glyph was drawn");
    assert!(red.0 > black.2, "black {black:?}, red {red:?}");
}

fn label(text: &str) -> Vec<Item> {
    let run = TextRun::new(text, Point::new(8.0, 8.0), 16.0, Color::BLACK);
    vec![Item::canvas(run)]
}

fn at(pan: Vec2) -> FrameView {
    frame(Camera::new(pan, 1.0))
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

fn send_view(harness: &mut Harness, frame: CpuFrame) {
    let result = harness
        .compositor
        .handle_page_event(frame_event(FrameLayer::View, frame));
    assert!(result.is_ok(), "frame rejected: {result:?}");
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
