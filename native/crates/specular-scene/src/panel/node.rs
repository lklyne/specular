//! One node of a panel: what is painted in its box, then its parts.

use specular_doc::TextFont;
use specular_interact::panel::builtin::{
    Chrome, Node, NodeState, PanelRect, Part, Pointing, Surface, Tint, Tone,
};
use specular_interact::{PaintRole, Palette};

use super::icons::{self, Inks};
use super::input;
use super::rect;
use crate::view::palette;
use crate::{
    Color, Colors, EllipseDraw, FontFamily, Item, PanelColors, Point, RectDraw, Stroke,
    StrokeAlign, TextAlign, TextRun, VerticalAlign,
};

/// A control that cannot be used: `disabled:opacity-30`.
const DISABLED: f32 = 0.3;
/// A row faded because its page has left its document: `opacity-50`.
const DIMMED: f32 = 0.5;

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

fn resolve(tint: &Tint, colors: &Colors) -> Color {
    let hues = match tint.palette {
        Palette::Soft => palette::Palette::Soft,
        Palette::Vivid => palette::Palette::Vivid,
    };
    let role = match tint.role {
        PaintRole::Fill => palette::Role::Fill,
        PaintRole::Ink => palette::Role::Ink,
    };
    palette::resolve(&tint.color, hues, role, colors)
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
fn fill(p: &PanelColors, chrome: Chrome, state: NodeState) -> Option<Color> {
    match (chrome, state.pointing) {
        (Chrome::ToolButton, _) => lit(state).then_some(p.tool_fill),
        (Chrome::ToolMenu, Pointing::Hover | Pointing::Pressed) if !state.on => Some(p.tool_fill),
        (Chrome::ToolMenu, _) => state.on.then_some(p.on),
        (Chrome::Button | Chrome::PresetRow, Pointing::Pressed) => Some(p.on),
        (Chrome::Button | Chrome::PresetRow, _) if state.on => Some(p.on),
        (Chrome::Button | Chrome::PresetRow, Pointing::Hover) => Some(p.hover),
        (Chrome::MenuRow, Pointing::Hover | Pointing::Pressed) => Some(p.menu_hover),
        (Chrome::Divider, _) => Some(p.divider),
        (Chrome::Rule, _) => Some(p.rule),
        (Chrome::Edge, _) => Some(p.sidebar_rule),
        (Chrome::Scrollbar, _) => Some(p.scroll_thumb),
        (Chrome::Row, _) if state.on => Some(p.interactive),
        (Chrome::Subtle, Pointing::Pressed) => Some(p.interactive),
        (Chrome::Row | Chrome::Subtle, Pointing::Hover | Pointing::Pressed) => {
            Some(p.interactive_hover)
        }
        (
            Chrome::Button | Chrome::PresetRow | Chrome::MenuRow | Chrome::Row | Chrome::Subtle,
            Pointing::Away,
        )
        | (
            Chrome::Plain | Chrome::Swatch | Chrome::Field | Chrome::Input | Chrome::InlineInput,
            _,
        ) => None,
    }
}

/// The color of a node's own text and glyphs: quiet at rest, full when the
/// pointer is on it or it is on.
fn follow(p: &PanelColors, surface: Surface, state: NodeState) -> Color {
    match (surface, lit(state)) {
        (Surface::Tabs | Surface::Toolbar, true) => p.toolbar_text_strong,
        (Surface::Tabs | Surface::Toolbar, false) => p.toolbar_text,
        (Surface::Dock | Surface::Dropdown | Surface::Sidebar | Surface::SidebarList, true) => {
            p.text
        }
        (Surface::Dock | Surface::Dropdown | Surface::Sidebar | Surface::SidebarList, false) => {
            p.text_muted
        }
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
pub(super) fn line(
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
fn dot_color(parts: &[Part], colors: &Colors) -> Option<Color> {
    parts.iter().find_map(|part| match part {
        Part::Dot { tint, .. } => tint.as_ref().map(|tint| resolve(tint, colors)),
        Part::Icon { .. }
        | Part::Text { .. }
        | Part::Chevron { .. }
        | Part::Check { .. }
        | Part::Key { .. }
        | Part::Glyph { .. }
        | Part::Input(_) => None,
    })
}

fn chrome(colors: &'static Colors, node: &Node, out: &mut Vec<Item>) {
    let p = &colors.panel;
    let area = rect(node.rect);
    if let Some(color) = fill(p, node.chrome, node.state) {
        out.push(Item::screen(
            RectDraw::filled(area, color).with_corner_radius(node.radius),
        ));
    }
    match node.chrome {
        Chrome::Swatch if node.state.on => {
            let ring = dot_color(&node.parts, colors)
                .filter(|color| luminance(*color) <= PALE)
                .unwrap_or(p.ring_gray);
            out.push(Item::screen(EllipseDraw {
                rect: area,
                fill: None,
                stroke: Some(Stroke::new(ring, RING, StrokeAlign::Inside)),
            }));
        }
        Chrome::Field => {
            let edge = Stroke::new(p.field_border, 1.0, StrokeAlign::Inside);
            out.push(Item::screen(
                RectDraw::outlined(area, edge).with_corner_radius(node.radius),
            ));
        }
        Chrome::Input => input::chrome(p, node, true, out),
        Chrome::InlineInput => input::chrome(p, node, false, out),
        Chrome::Swatch
        | Chrome::Plain
        | Chrome::ToolButton
        | Chrome::ToolMenu
        | Chrome::Button
        | Chrome::MenuRow
        | Chrome::PresetRow
        | Chrome::Divider
        | Chrome::Rule
        | Chrome::Row
        | Chrome::Subtle
        | Chrome::Edge
        | Chrome::Scrollbar => {}
    }
}

fn part(colors: &'static Colors, surface: Surface, node: &Node, part: &Part, out: &mut Vec<Item>) {
    let p = &colors.panel;
    let own = follow(p, surface, node.state);
    match part {
        Part::Icon {
            icon,
            rect: area,
            tint,
        } => {
            let inks = Inks {
                current: own,
                tint: tint.as_ref().map(|tint| resolve(tint, colors)),
                on: node.state.on,
                colors,
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
                Tone::Strong => p.text,
                Tone::Muted => p.text_muted,
            };
            out.push(line(text, *area, *align, *font, *weight, color));
        }
        Part::Dot { rect: area, tint } => {
            let (color, edge) = match tint {
                Some(tint) => (resolve(tint, colors), p.dot_edge),
                None => (p.popup, p.popup_border),
            };
            out.push(Item::screen(
                EllipseDraw::filled(rect(*area), color).with_stroke(Stroke::new(
                    edge,
                    1.0,
                    StrokeAlign::Inside,
                )),
            ));
        }
        Part::Glyph {
            icon,
            rect: area,
            tone,
        } => {
            let color = match tone {
                Tone::Follow => own,
                Tone::Strong => p.text,
                Tone::Muted => p.text_muted,
            };
            icons::draw(*icon, rect(*area), Inks::plain(color, colors), out);
        }
        Part::Chevron { rect: area } => {
            let color = match surface {
                Surface::Tabs | Surface::Toolbar => p.toolbar_chevron,
                Surface::Dock | Surface::Dropdown | Surface::Sidebar | Surface::SidebarList => own,
            };
            icons::chevron(rect(*area), color, colors, out);
        }
        Part::Check { rect: area } => icons::check(rect(*area), p.text, colors, out),
        Part::Input(input) => input::draw(colors, input, out),
        Part::Key { text, rect: area } => {
            out.push(Item::screen(
                RectDraw::filled(rect(*area), p.key).with_corner_radius(KEY_RADIUS),
            ));
            out.push(line(
                text,
                *area,
                specular_doc::TextAlign::Center,
                TextFont::Sans,
                400,
                p.key_text,
            ));
        }
    }
}

/// The items of `node`, which is on a panel of `surface`.
pub(super) fn draw(colors: &'static Colors, surface: Surface, node: &Node, out: &mut Vec<Item>) {
    let first = out.len();
    chrome(colors, node, out);
    for it in &node.parts {
        part(colors, surface, node, it, out);
    }
    let fade = match (node.state.enabled, node.state.dimmed) {
        (false, _) => DISABLED,
        (true, true) => DIMMED,
        (true, false) => return,
    };
    for item in out.iter_mut().skip(first) {
        item.opacity *= fade;
    }
}
