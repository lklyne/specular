//! A row of controls: the content of a popup, and of a dropdown section that
//! holds controls.

use glam::Vec2;
use specular_doc::{TextAlign, TextFont};

use super::super::{
    Button, Control, ControlId, Face, Icon, PaintRole, Palette, Stepper, Swatches, Toggle,
};
use super::metrics::{
    CONTROL, CONTROL_RADIUS, DIVIDER, DIVIDER_MARGIN, DOT, GAP, ICON, RULE, RULE_MARGIN,
    SEGMENT_PAD, STEPPER_VALUE, SWATCH,
};
use super::node::{Chrome, Node, PanelRect, Part, Run, Tint, Tone};
use super::{Ctx, trigger};

/// The glyph of the choice that paints nothing, inside its ring.
pub(super) const BAN: f32 = 12.0;
/// Regular and medium on the CSS scale.
pub(super) const REGULAR: u16 = 400;
pub(super) const MEDIUM: u16 = 500;

/// Where a row is, which decides the line drawn for a separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowKind {
    /// The popup's own row.
    Popup,
    /// A row inside a dropdown.
    Dropdown,
}

/// A row laid out from its own top-left corner.
pub(super) struct Row {
    pub(super) nodes: Vec<Node>,
    pub(super) width: f32,
}

/// The box a glyph is fitted into on a popup control. The stroke samples
/// are drawn at the size they were designed at.
fn icon_box(icon: Icon) -> Vec2 {
    match icon {
        Icon::StrokeThin => Vec2::new(17.0, 9.0),
        Icon::StrokeThick => Vec2::new(19.0, 11.0),
        Icon::SelectTool
        | Icon::PageTool
        | Icon::TextTool
        | Icon::StickyTool
        | Icon::DocumentTool
        | Icon::ShapeTool
        | Icon::DrawPenTool
        | Icon::DrawHighlightTool
        | Icon::CommentTool
        | Icon::Shape(_)
        | Icon::AlignLeft
        | Icon::AlignCenter
        | Icon::AlignRight
        | Icon::BrushPen
        | Icon::BrushHighlighter
        | Icon::Border
        | Icon::LineSolid
        | Icon::LineDashed
        | Icon::Ban
        | Icon::ArrowStart
        | Icon::ArrowEnd
        | Icon::Trash
        | Icon::Bold
        | Icon::Strikethrough
        | Icon::BulletList
        | Icon::Device
        | Icon::Rotate
        | Icon::SchemeSystem
        | Icon::SchemeLight
        | Icon::SchemeDark => Vec2::splat(ICON),
    }
}

/// A glyph that shows a color is a pen or a highlighter, whose tip takes
/// the stroke's ink.
fn ink(face: &Face) -> Option<Tint> {
    face.color.clone().map(|color| Tint {
        color,
        palette: Palette::Vivid,
        role: PaintRole::Ink,
    })
}

pub(super) fn text(
    label: impl Into<super::super::Label>,
    rect: PanelRect,
    align: TextAlign,
    font: Option<TextFont>,
    tone: Tone,
) -> Part {
    Part::Text {
        text: label.into(),
        rect,
        align,
        font: font.unwrap_or(TextFont::Sans),
        weight: REGULAR,
        tone,
    }
}

fn face_width(ctx: &Ctx<'_>, face: &Face) -> f32 {
    match (&face.icon, &face.text) {
        (Some(_), Some(label)) => {
            SEGMENT_PAD * 2.0 + ICON + GAP + ctx.text_width(label, TextFont::Sans)
        }
        (None, Some(label)) => {
            CONTROL.max(ctx.text_width(label, TextFont::Sans) + SEGMENT_PAD * 2.0)
        }
        (_, None) => CONTROL,
    }
}

/// The parts of a button's face, centred in `rect`.
fn face_parts(ctx: &Ctx<'_>, face: &Face, rect: PanelRect) -> Vec<Part> {
    match (face.icon, &face.text) {
        (Some(icon), Some(label)) => {
            let label_width = ctx.text_width(label, TextFont::Sans);
            let left = rect.centre().x - (ICON + GAP + label_width) / 2.0;
            let glyph = PanelRect::new(left, rect.centre().y - ICON / 2.0, ICON, ICON);
            let words = PanelRect::new(left + ICON + GAP, rect.y, label_width, rect.height);
            vec![
                Part::Icon {
                    icon,
                    rect: glyph,
                    tint: ink(face),
                },
                text(
                    label.clone(),
                    words,
                    TextAlign::Left,
                    face.font,
                    Tone::Follow,
                ),
            ]
        }
        (Some(icon), None) => vec![Part::Icon {
            icon,
            rect: rect.centred(icon_box(icon)),
            tint: ink(face),
        }],
        (None, Some(label)) => vec![text(
            label.clone(),
            rect,
            TextAlign::Center,
            face.font,
            Tone::Follow,
        )],
        (None, None) => vec![Part::Dot {
            rect: rect.centred(Vec2::splat(DOT)),
            tint: ink(face),
        }],
    }
}

pub(super) fn pressable(
    ctx: &Ctx<'_>,
    id: &ControlId,
    rect: PanelRect,
    enabled: bool,
    on: bool,
) -> Node {
    Node {
        id: Some(id.clone()),
        rect,
        radius: CONTROL_RADIUS,
        chrome: Chrome::Button,
        state: ctx.state(id, enabled, on),
        parts: Vec::new(),
        run: None,
    }
}

fn button(ctx: &Ctx<'_>, button: &Button, rect: PanelRect) -> Node {
    Node {
        parts: face_parts(ctx, &button.face, rect),
        run: Some(Run::Act {
            action: button.action.clone(),
            closes: false,
        }),
        ..pressable(ctx, &button.id, rect, button.enabled, false)
    }
}

fn toggle(ctx: &Ctx<'_>, toggle: &Toggle, rect: PanelRect) -> Node {
    Node {
        parts: face_parts(ctx, &toggle.face, rect),
        run: Some(Run::Act {
            action: toggle.action.clone(),
            closes: false,
        }),
        ..pressable(ctx, &toggle.id, rect, toggle.enabled, toggle.on)
    }
}

fn swatches_width(swatches: &Swatches, gap: f32) -> f32 {
    let count = swatches.options.len() as f32;
    count * SWATCH + (count - 1.0).max(0.0) * gap
}

fn swatches(ctx: &Ctx<'_>, row: &Swatches, left: f32, gap: f32, out: &mut Vec<Node>) {
    for (index, swatch) in row.options.iter().enumerate() {
        let x = left + index as f32 * (SWATCH + gap);
        let rect = PanelRect::new(x, (CONTROL - SWATCH) / 2.0, SWATCH, SWATCH);
        let tint = swatch.color.clone().map(|color| Tint {
            color,
            palette: row.palette,
            role: row.role,
        });
        let mut parts = vec![Part::Dot {
            rect: rect.centred(Vec2::splat(DOT)),
            tint: tint.clone(),
        }];
        if tint.is_none() {
            parts.push(Part::Icon {
                icon: Icon::Ban,
                rect: rect.centred(Vec2::splat(BAN)),
                tint: None,
            });
        }
        out.push(Node {
            radius: SWATCH / 2.0,
            chrome: Chrome::Swatch,
            parts,
            run: Some(Run::Act {
                action: swatch.action.clone(),
                closes: false,
            }),
            ..pressable(ctx, &swatch.id, rect, row.enabled, swatch.selected)
        });
    }
}

fn stepper_width(ctx: &Ctx<'_>, stepper: &Stepper) -> f32 {
    let value = ctx.text_width(&stepper.value, TextFont::Sans) + SEGMENT_PAD * 2.0;
    CONTROL * 2.0 + value.max(STEPPER_VALUE)
}

/// A number in a box with a button at each end.
fn stepper(ctx: &Ctx<'_>, stepper: &Stepper, left: f32, width: f32, out: &mut Vec<Node>) {
    let field = PanelRect::new(left, 0.0, width, CONTROL);
    out.push(Node {
        radius: CONTROL_RADIUS,
        parts: vec![text(
            stepper.value.clone(),
            field,
            TextAlign::Center,
            None,
            Tone::Strong,
        )],
        ..Node::fixed(field, Chrome::Field)
    });
    let halves = [
        (
            "dec",
            "\u{2212}",
            left,
            stepper.can_decrement,
            &stepper.decrement,
        ),
        (
            "inc",
            "+",
            field.right() - CONTROL,
            stepper.can_increment,
            &stepper.increment,
        ),
    ];
    for (name, sign, x, enabled, action) in halves {
        let id = stepper.id.child(name);
        let rect = PanelRect::new(x, 0.0, CONTROL, CONTROL);
        out.push(Node {
            parts: vec![text(sign, rect, TextAlign::Center, None, Tone::Follow)],
            run: Some(Run::Act {
                action: action.clone(),
                closes: false,
            }),
            ..pressable(ctx, &id, rect, enabled, false)
        });
    }
}

fn separator_width(kind: RowKind) -> f32 {
    match kind {
        RowKind::Popup => DIVIDER.0 + DIVIDER_MARGIN * 2.0,
        RowKind::Dropdown => RULE.0 + RULE_MARGIN * 2.0,
    }
}

fn separator(kind: RowKind, left: f32) -> Node {
    let (chrome, (width, height), margin) = match kind {
        RowKind::Popup => (Chrome::Divider, DIVIDER, DIVIDER_MARGIN),
        RowKind::Dropdown => (Chrome::Rule, RULE, RULE_MARGIN),
    };
    let rect = PanelRect::new(left + margin, (CONTROL - height) / 2.0, width, height);
    Node::fixed(rect, chrome)
}

fn natural(ctx: &Ctx<'_>, control: &Control, kind: RowKind) -> f32 {
    match control {
        Control::Button(button) => face_width(ctx, &button.face),
        Control::Toggle(toggle) => face_width(ctx, &toggle.face),
        Control::Swatches(swatches) => swatches_width(swatches, GAP),
        Control::Dropdown(dropdown) => trigger::width(ctx, dropdown),
        Control::Stepper(stepper) => stepper_width(ctx, stepper),
        Control::Separator => separator_width(kind),
    }
}

/// Whether a control takes a share of the room left over in a row that is
/// wider than its content: a labelled toggle (`flex-1`), a stepper.
fn stretches(control: &Control) -> bool {
    match control {
        Control::Toggle(toggle) => toggle.face.icon.is_some() && toggle.face.text.is_some(),
        Control::Stepper(_) => true,
        Control::Button(_) | Control::Swatches(_) | Control::Dropdown(_) | Control::Separator => {
            false
        }
    }
}

/// The width `controls` need side by side.
pub(super) fn natural_width(ctx: &Ctx<'_>, controls: &[Control], kind: RowKind) -> f32 {
    let widths: f32 = controls.iter().map(|it| natural(ctx, it, kind)).sum();
    widths + controls.len().saturating_sub(1) as f32 * GAP
}

/// `controls` side by side, each `CONTROL` tall. With `fill`, the row is
/// that wide: the controls that stretch share what is left over, and a row
/// that is only swatches spreads them from edge to edge.
pub(super) fn row(ctx: &Ctx<'_>, controls: &[Control], kind: RowKind, fill: Option<f32>) -> Row {
    let content = natural_width(ctx, controls, kind);
    // Negative when the row is narrower than its content: the controls that
    // stretch give up their padding to fit.
    let slack = fill.map_or(0.0, |fill| fill - content);
    let spare = slack.max(0.0);
    let stretching = controls.iter().filter(|it| stretches(it)).count();
    let share = if stretching == 0 {
        0.0
    } else {
        slack / stretching as f32
    };
    let mut nodes = Vec::new();
    let mut left = 0.0;
    for control in controls {
        let own = natural(ctx, control, kind);
        let mut width = if stretches(control) {
            (own + share).max(own - SEGMENT_PAD * 2.0)
        } else {
            own
        };
        let rect = PanelRect::new(left, 0.0, width, CONTROL);
        match control {
            Control::Button(it) => nodes.push(button(ctx, it, rect)),
            Control::Toggle(it) => nodes.push(toggle(ctx, it, rect)),
            Control::Swatches(it) => {
                let between = it.options.len().saturating_sub(1).max(1) as f32;
                let gap = match (controls.len(), stretching) {
                    (1, 0) => GAP + spare / between,
                    _ => GAP,
                };
                swatches(ctx, it, left, gap, &mut nodes);
                width = swatches_width(it, gap);
            }
            Control::Dropdown(it) => nodes.push(trigger::node(ctx, it, left)),
            Control::Stepper(it) => stepper(ctx, it, left, width, &mut nodes),
            Control::Separator => nodes.push(separator(kind, left)),
        }
        left += width + GAP;
    }
    Row {
        nodes,
        width: (left - GAP).max(0.0),
    }
}
