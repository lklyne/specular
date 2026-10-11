//! The agent cursors, over everything else the canvas draws: an arrow at
//! the point an agent is acting on and a chip saying what it is doing. Both
//! keep their pixel size at any zoom.

use specular_interact::{AgentCursor, InspectPopover};

use super::frame::Frame;
use crate::{
    Color, FontFamily, Item, PathStroke, Point, PolygonDraw, Rect, RectDraw, Scene, TextRun,
    VerticalAlign,
};

/// The arrow's corners from its tip, in logical pixels.
const ARROW: [(f32, f32); 4] = [(0.0, 0.0), (0.0, 16.0), (4.4, 12.2), (11.6, 11.6)];
const ARROW_EDGE: f32 = 1.5;
/// The chip's top-left corner from the tip.
const CHIP_OFFSET: (f32, f32) = (10.0, 16.0);
const CHIP_HEIGHT: f32 = 18.0;
const CHIP_PADDING: f32 = 6.0;
const CHIP_RADIUS: f32 = 9.0;
/// One colour a session, in the order sessions first act.
const COLORS: [Color; 6] = [
    Color::rgba(59, 130, 246, 255),
    Color::rgba(139, 92, 246, 255),
    Color::rgba(236, 72, 153, 255),
    Color::rgba(249, 115, 22, 255),
    Color::rgba(34, 197, 94, 255),
    Color::rgba(6, 182, 212, 255),
];

pub(crate) fn draw(frame: &Frame<'_>, scene: &mut Scene) {
    for cursor in frame.app.agent_cursors() {
        draw_cursor(frame, &cursor, scene);
    }
}

fn draw_cursor(frame: &Frame<'_>, cursor: &AgentCursor, scene: &mut Scene) {
    let tip = frame.screen_point(cursor.at);
    let width = InspectPopover::text_width(cursor.label) + 2.0 * CHIP_PADDING;
    let chip = Rect::new(
        tip.x + CHIP_OFFSET.0,
        tip.y + CHIP_OFFSET.1,
        width,
        CHIP_HEIGHT,
    );
    let reach = Rect::new(
        tip.x,
        tip.y,
        chip.x + chip.width - tip.x,
        chip.y + chip.height - tip.y,
    );
    if !frame.sees_screen(reach) {
        return;
    }
    let color = COLORS[cursor.slot % COLORS.len()];
    scene.push(Item::screen(PolygonDraw {
        points: (ARROW.iter())
            .map(|(x, y)| Point::new(tip.x + x, tip.y + y))
            .collect(),
        fill: Some(color),
        stroke: Some(PathStroke::new(Color::WHITE, ARROW_EDGE)),
    }));
    scene.push(Item::screen(
        RectDraw::filled(chip, color).with_corner_radius(CHIP_RADIUS),
    ));
    scene.push(Item::screen(TextRun {
        family: FontFamily::Monospace,
        line_height: InspectPopover::LINE,
        box_height: Some(CHIP_HEIGHT),
        vertical_align: VerticalAlign::Middle,
        ..TextRun::new(
            cursor.label,
            Point::new(chip.x + CHIP_PADDING, chip.y),
            InspectPopover::FONT_SIZE,
            Color::WHITE,
        )
    }));
}
