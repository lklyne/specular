//! A row of controls: the content of the dock, and of a dropdown section
//! that holds controls.

use glam::Vec2;
use specular_doc::{TextAlign, TextFont};

use super::super::{
    Button, Control, ControlId, Face, FieldWidth, Icon, PaintRole, Palette, Stepper, Swatches,
    Toggle,
};
use super::metrics::{
    CONTROL, CONTROL_RADIUS, DIVIDER, DIVIDER_MARGIN, DOT, FIELD_HEIGHT, GAP, ICON, RULE,
    RULE_MARGIN, SEGMENT_PAD, STEPPER_VALUE, SWATCH,
};
use super::node::{Chrome, Node, PanelRect, Part, Run, Tint, Tone};
use super::{Ctx, field, trigger};

/// The glyph of the choice that paints nothing, inside its ring.
pub(super) const BAN: f32 = 12.0;
/// Regular and medium on the CSS scale.
pub(super) const REGULAR: u16 = 400;
pub(super) const MEDIUM: u16 = 500;

/// Where a row is, which decides the line drawn for a separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowKind {
    /// The dock's row.
    Dock,
    /// A row inside a dropdown.
    Dropdown,
}

/// A row laid out from its own top-left corner.
pub(super) struct Row {
    pub(super) nodes: Vec<Node>,
    pub(super) height: f32,
}

/// The glyph of reload and stop: `<RotateCw size={12} />`.
const RELOAD_ICON: f32 = 12.0;

/// The box a glyph is fitted into on a dock control. The stroke samples
/// are drawn at the size they were designed at.
fn icon_box(icon: Icon) -> Vec2 {
    match icon {
        Icon::StrokeThin => Vec2::new(17.0, 9.0),
        Icon::StrokeThick => Vec2::new(19.0, 11.0),
        Icon::Reload | Icon::Stop => Vec2::splat(RELOAD_ICON),
        _ => Vec2::splat(ICON),
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
        RowKind::Dock => DIVIDER.0 + DIVIDER_MARGIN * 2.0,
        RowKind::Dropdown => RULE.0 + RULE_MARGIN * 2.0,
    }
}

fn separator(kind: RowKind, left: f32) -> Node {
    let (chrome, (width, height), margin) = match kind {
        RowKind::Dock => (Chrome::Divider, DIVIDER, DIVIDER_MARGIN),
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
        Control::Field(it) => field::natural_width(ctx, it),
        // A choice list is a context menu, which is laid out as a list.
        Control::Choices(_) => 0.0,
        Control::Separator => separator_width(kind),
    }
}

/// Whether a control takes a share of the room left over in a row that is
/// wider than its content. In a dropdown: a labelled toggle (`flex-1`), a
/// stepper, a field (`min-w-0 flex-1`). In the dock only an address does;
/// everything else keeps its size.
fn stretches(control: &Control, kind: RowKind) -> bool {
    match kind {
        RowKind::Dock => {
            matches!(control, Control::Field(field) if field.width == FieldWidth::Wide)
        }
        RowKind::Dropdown => match control {
            Control::Toggle(toggle) => toggle.face.icon.is_some() && toggle.face.text.is_some(),
            Control::Stepper(_) | Control::Field(_) => true,
            Control::Button(_)
            | Control::Swatches(_)
            | Control::Dropdown(_)
            | Control::Choices(_)
            | Control::Separator => false,
        },
    }
}

/// The width `controls` need side by side.
pub(super) fn natural_width(ctx: &Ctx<'_>, controls: &[Control], kind: RowKind) -> f32 {
    let widths: f32 = controls.iter().map(|it| natural(ctx, it, kind)).sum();
    widths + controls.len().saturating_sub(1) as f32 * GAP
}

/// `controls` side by side, each `CONTROL` tall, or `FIELD_HEIGHT` when the
/// row holds a field, which the rest are centred against. With `fill`, the
/// row is that wide: the controls that stretch share what is left over, and
/// a dropdown's row that is only swatches spreads them from edge to edge.
pub(super) fn row(ctx: &Ctx<'_>, controls: &[Control], kind: RowKind, fill: Option<f32>) -> Row {
    let content = natural_width(ctx, controls, kind);
    // Negative when the row is narrower than its content: the controls that
    // stretch give up their padding to fit.
    let slack = fill.map_or(0.0, |fill| fill - content);
    let spare = slack.max(0.0);
    let stretching = (controls.iter()).filter(|it| stretches(it, kind)).count();
    let share = if stretching == 0 {
        0.0
    } else {
        slack / stretching as f32
    };
    let has_field = controls
        .iter()
        .any(|control| matches!(control, Control::Field(_)));
    let height = if has_field { FIELD_HEIGHT } else { CONTROL };
    let lift = Vec2::new(0.0, (height - CONTROL) / 2.0);
    let mut nodes = Vec::new();
    let mut left = 0.0;
    for control in controls {
        let own = natural(ctx, control, kind);
        let stretches = stretches(control, kind);
        let mut width = if !stretches {
            own
        } else if matches!(control, Control::Field(_)) {
            // An address never gives up the room it asks for.
            own + share.max(0.0)
        } else {
            (own + share).max(own - SEGMENT_PAD * 2.0)
        };
        let rect = PanelRect::new(left, 0.0, width, CONTROL);
        let first = nodes.len();
        match control {
            Control::Button(it) => nodes.push(button(ctx, it, rect)),
            Control::Toggle(it) => nodes.push(toggle(ctx, it, rect)),
            Control::Swatches(it) => {
                let between = it.options.len().saturating_sub(1).max(1) as f32;
                let gap = match (kind, controls.len(), stretching) {
                    (RowKind::Dropdown, 1, 0) => GAP + spare / between,
                    _ => GAP,
                };
                swatches(ctx, it, left, gap, &mut nodes);
                width = swatches_width(it, gap);
            }
            Control::Dropdown(it) => nodes.push(trigger::node(ctx, it, left)),
            Control::Stepper(it) => stepper(ctx, it, left, width, &mut nodes),
            Control::Field(it) => {
                field::nodes(ctx, it, left, width, &mut nodes);
                left += width + GAP;
                continue;
            }
            Control::Choices(_) => {}
            Control::Separator => nodes.push(separator(kind, left)),
        }
        for node in &mut nodes[first..] {
            *node = node.clone().moved(lift);
        }
        left += width + GAP;
    }
    Row { nodes, height }
}
