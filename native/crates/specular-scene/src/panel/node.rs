//! One node of a panel: what is painted in its box, then its parts.

use specular_doc::TextFont;
use specular_interact::panel::builtin::{
    Chrome, Node, NodeState, PanelRect, Part, Pointing, Surface, Tint, Tone,
};
use specular_interact::{PaintRole, Palette};

use super::colors::{
    DISABLED, DIVIDER, DOT_EDGE, FIELD_BORDER, HOVER, KEY, KEY_TEXT, MENU_HOVER, ON, POPUP,
    POPUP_BORDER, RING_GRAY, RULE, TEXT, TEXT_MUTED, TOOL_FILL, TOOLBAR_CHEVRON, TOOLBAR_TEXT,
    TOOLBAR_TEXT_STRONG,
};
use super::icons::{self, Inks};
use super::rect;
use crate::view::palette;
use crate::{
    Color, EllipseDraw, FontFamily, Item, Point, RectDraw, Stroke, StrokeAlign, TextAlign, TextRun,
    VerticalAlign,
};

/// Panel text: `text-xs`, 12 px on a 16 px line.
const TEXT_SIZE: f32 = 12.0;
const TEXT_LINE: f32 = 16.0;
/// The corner of a key hint: `rounded-[4px]`.
const KEY_RADIUS: f32 = 4.0;
/// The ring of a selected swatch: `inset 0 0 0 2px`.
const RING: f32 = 2.0;
/// Above this a swatch is too near white to ring in its own color
/// (`swatchRingColor` in `colorSwatchStyle.ts`).
const PALE: f32 = 0.92;

fn resolve(tint: &Tint) -> Color {
    let hues = match tint.palette {
        Palette::Soft => palette::Palette::Soft,
        Palette::Vivid => palette::Palette::Vivid,
    };
    let role = match tint.role {
        PaintRole::Fill => palette::Role::Fill,
        PaintRole::Ink => palette::Role::Ink,
    };
    palette::resolve(&tint.color, hues, role)
}

/// How light `color` looks, 0 to 1.
fn luminance(color: Color) -> f32 {
    let channel = |value: u8| f32::from(value) / 255.0;
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

fn lit(state: NodeState) -> bool {
    state.on
        || match state.pointing {
            Pointing::Hover | Pointing::Pressed => true,
            Pointing::Away => false,
        }
}

/// The fill behind a node's parts, if it has one now.
fn fill(chrome: Chrome, state: NodeState) -> Option<Color> {
    match (chrome, state.pointing) {
        (Chrome::ToolButton, _) => lit(state).then_some(TOOL_FILL),
        (Chrome::ToolMenu, Pointing::Hover | Pointing::Pressed) if !state.on => Some(TOOL_FILL),
        (Chrome::ToolMenu, _) => state.on.then_some(ON),
        (Chrome::Button | Chrome::PresetRow, Pointing::Pressed) => Some(ON),
        (Chrome::Button | Chrome::PresetRow, _) if state.on => Some(ON),
        (Chrome::Button | Chrome::PresetRow, Pointing::Hover) => Some(HOVER),
        (Chrome::MenuRow, Pointing::Hover | Pointing::Pressed) => Some(MENU_HOVER),
        (Chrome::Divider, _) => Some(DIVIDER),
        (Chrome::Rule, _) => Some(RULE),
        (Chrome::Button | Chrome::PresetRow | Chrome::MenuRow, Pointing::Away)
        | (Chrome::Plain | Chrome::Swatch | Chrome::Field, _) => None,
    }
}

/// The color of a node's own text and glyphs: quiet at rest, full when the
/// pointer is on it or it is on.
fn follow(surface: Surface, state: NodeState) -> Color {
    match (surface, lit(state)) {
        (Surface::Toolbar, true) => TOOLBAR_TEXT_STRONG,
        (Surface::Toolbar, false) => TOOLBAR_TEXT,
        (Surface::Popup | Surface::Dropdown, true) => TEXT,
        (Surface::Popup | Surface::Dropdown, false) => TEXT_MUTED,
    }
}

fn family(font: TextFont) -> FontFamily {
    match font {
        TextFont::Sans => FontFamily::SansSerif,
        TextFont::Mono => FontFamily::Monospace,
        TextFont::Hand => FontFamily::Named("Kalam".to_owned()),
    }
}

/// One line of `text`, vertically centred in `area`.
fn line(
    text: &str,
    area: PanelRect,
    align: specular_doc::TextAlign,
    font: TextFont,
    weight: u16,
    color: Color,
) -> Item {
    let (x, align) = match align {
        specular_doc::TextAlign::Left => (area.x, TextAlign::Left),
        specular_doc::TextAlign::Center => (area.centre().x, TextAlign::Centre),
        specular_doc::TextAlign::Right => (area.right(), TextAlign::Right),
    };
    Item::screen(TextRun {
        box_height: Some(area.height),
        family: family(font),
        line_height: TEXT_LINE,
        weight,
        align,
        vertical_align: VerticalAlign::Middle,
        ..TextRun::new(text, Point::new(x, area.y), TEXT_SIZE, color)
    })
}

/// The color of the dot among `parts`, which a swatch's ring takes.
fn dot_color(parts: &[Part]) -> Option<Color> {
    parts.iter().find_map(|part| match part {
        Part::Dot { tint, .. } => tint.as_ref().map(resolve),
        Part::Icon { .. }
        | Part::Text { .. }
        | Part::Chevron { .. }
        | Part::Check { .. }
        | Part::Key { .. } => None,
    })
}

fn chrome(node: &Node, out: &mut Vec<Item>) {
    let area = rect(node.rect);
    if let Some(color) = fill(node.chrome, node.state) {
        out.push(Item::screen(
            RectDraw::filled(area, color).with_corner_radius(node.radius),
        ));
    }
    match node.chrome {
        Chrome::Swatch if node.state.on => {
            let ring = dot_color(&node.parts)
                .filter(|color| luminance(*color) <= PALE)
                .unwrap_or(RING_GRAY);
            out.push(Item::screen(EllipseDraw {
                rect: area,
                fill: None,
                stroke: Some(Stroke::new(ring, RING, StrokeAlign::Inside)),
            }));
        }
        Chrome::Field => {
            let edge = Stroke::new(FIELD_BORDER, 1.0, StrokeAlign::Inside);
            out.push(Item::screen(
                RectDraw::outlined(area, edge).with_corner_radius(node.radius),
            ));
        }
        Chrome::Swatch
        | Chrome::Plain
        | Chrome::ToolButton
        | Chrome::ToolMenu
        | Chrome::Button
        | Chrome::MenuRow
        | Chrome::PresetRow
        | Chrome::Divider
        | Chrome::Rule => {}
    }
}

fn part(surface: Surface, node: &Node, part: &Part, out: &mut Vec<Item>) {
    let own = follow(surface, node.state);
    match part {
        Part::Icon {
            icon,
            rect: area,
            tint,
        } => {
            let inks = Inks {
                current: own,
                tint: tint.as_ref().map(resolve),
                on: node.state.on,
            };
            icons::draw(*icon, rect(*area), inks, out);
        }
        Part::Text {
            text,
            rect: area,
            align,
            font,
            weight,
            tone,
        } => {
            let color = match tone {
                Tone::Follow => own,
                Tone::Strong => TEXT,
                Tone::Muted => TEXT_MUTED,
            };
            out.push(line(text, *area, *align, *font, *weight, color));
        }
        Part::Dot { rect: area, tint } => {
            let (color, edge) = match tint {
                Some(tint) => (resolve(tint), DOT_EDGE),
                None => (POPUP, POPUP_BORDER),
            };
            out.push(Item::screen(
                EllipseDraw::filled(rect(*area), color).with_stroke(Stroke::new(
                    edge,
                    1.0,
                    StrokeAlign::Inside,
                )),
            ));
        }
        Part::Chevron { rect: area } => {
            let color = match surface {
                Surface::Toolbar => TOOLBAR_CHEVRON,
                Surface::Popup | Surface::Dropdown => own,
            };
            icons::chevron(rect(*area), color, out);
        }
        Part::Check { rect: area } => icons::check(rect(*area), TEXT, out),
        Part::Key { text, rect: area } => {
            out.push(Item::screen(
                RectDraw::filled(rect(*area), KEY).with_corner_radius(KEY_RADIUS),
            ));
            out.push(line(
                text,
                *area,
                specular_doc::TextAlign::Center,
                TextFont::Sans,
                400,
                KEY_TEXT,
            ));
        }
    }
}

/// The items of `node`, which is on a panel of `surface`.
pub(super) fn draw(surface: Surface, node: &Node, out: &mut Vec<Item>) {
    let first = out.len();
    chrome(node, out);
    for it in &node.parts {
        part(surface, node, it, out);
    }
    if !node.state.enabled {
        for item in out.iter_mut().skip(first) {
            item.opacity *= DISABLED;
        }
    }
}
