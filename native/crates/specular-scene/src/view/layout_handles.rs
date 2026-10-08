//! A line's handles (ADR 0015), in screen space: a pink bar in each gap
//! and a pink ring at the middle of each box, which fills out under the
//! pointer to the square it is grabbed in. And the box a reorder is
//! carrying, drawn again at half strength where the pointer has taken it,
//! while its slot shows where it would land.

use specular_interact::{DOT_RADIUS, LayoutAxis, ReorderGhost};

use super::frame::Frame;
use crate::{Color, EllipseDraw, Item, Rect, RectDraw, Scene, Space, Stroke, StrokeAlign};

const COLOR: Color = Color::rgb(0xec, 0x48, 0x99);
const RING_WIDTH: f32 = 2.5;
/// A gap bar: this long across the gap's axis and this thick, with a white
/// edge so it reads over anything.
const BAR_LENGTH: f32 = 24.0;
const BAR_THICKNESS: f32 = 2.0;
const BAR_EDGE: f32 = 1.0;
const BAR_REST_OPACITY: f32 = 0.6;
const GHOST_OPACITY: f32 = 0.5;

pub(crate) fn draw(frame: &Frame<'_>, scene: &mut Scene) {
    let app = frame.app;
    if let Some(ghost) = app.reorder_ghost() {
        push_ghost(frame, &ghost, scene);
    }
    for strip in app.gap_handles() {
        let centre = strip.rect.centre();
        let (width, height) = match strip.axis {
            LayoutAxis::X => (BAR_THICKNESS, BAR_LENGTH),
            LayoutAxis::Y => (BAR_LENGTH, BAR_THICKNESS),
        };
        let bar = Rect::new(
            centre.x - width / 2.0,
            centre.y - height / 2.0,
            width,
            height,
        );
        let edge = Stroke::new(Color::WHITE, BAR_EDGE, StrokeAlign::Outside);
        let opacity = if strip.hovered { 1.0 } else { BAR_REST_OPACITY };
        scene.push(
            Item::screen(RectDraw::filled(bar, COLOR).with_stroke(edge)).with_opacity(opacity),
        );
    }
    for dot in app.reorder_dots() {
        let round = |radius: f32| {
            Rect::new(
                dot.centre.x - radius,
                dot.centre.y - radius,
                radius * 2.0,
                radius * 2.0,
            )
        };
        let draw = if dot.hovered {
            EllipseDraw::filled(round(dot.hit / 2.0), COLOR)
        } else {
            let ring = Stroke::new(COLOR, RING_WIDTH, StrokeAlign::Centre);
            EllipseDraw {
                fill: None,
                ..EllipseDraw::filled(round(DOT_RADIUS.min(dot.hit / 2.0)), COLOR)
            }
            .with_stroke(ring)
        };
        scene.push(Item::screen(draw));
    }
}

/// The carried box where the pointer has it. `view` is a pure function of
/// the entity, so the ghost is the real thing.
fn push_ghost(frame: &Frame<'_>, ghost: &ReorderGhost, scene: &mut Scene) {
    let Some(entity) = (frame.app.document().entity(&ghost.entity))
        .and_then(|entity| specular_interact::seen(frame.app, entity))
    else {
        return;
    };
    let (dx, dy) = (ghost.delta.x as f32, ghost.delta.y as f32);
    let mut drawn = Scene::new();
    super::draw_entity(&frame.without_chrome(), &entity.entity, &mut drawn);
    scene.extend(drawn.items.into_iter().map(|item| {
        let by = match item.space {
            Space::Canvas => 1.0,
            Space::Screen => frame.zoom(),
        };
        let opacity = item.opacity * GHOST_OPACITY;
        item.translated(dx * by, dy * by).with_opacity(opacity)
    }));
}
