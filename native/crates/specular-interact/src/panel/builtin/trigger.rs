//! A closed dropdown: the control that shows the current value and opens
//! the list.

use glam::Vec2;
use specular_doc::{TextAlign, TextFont};

use super::super::{Control, Dropdown, DropdownSection, Icon, PaintRole, Palette};
use super::Ctx;
use super::controls::{BAN, pressable, text};
use super::metrics::{CHEVRON, CONTROL, GAP, ICON, PRESET_CHEVRON, TRIGGER_DOT, TRIGGER_SHAPE};
use super::node::{Node, PanelRect, Part, Run, Tint, Tone};

/// The palette and role the swatches inside `dropdown` paint in, which the
/// dot on its trigger shares.
fn swatch_style(dropdown: &Dropdown) -> (Palette, PaintRole) {
    (dropdown.content.iter())
        .find_map(|section| match section {
            DropdownSection::Controls(controls) => {
                controls.iter().find_map(|control| match control {
                    Control::Swatches(swatches) => Some((swatches.palette, swatches.role)),
                    Control::Button(_)
                    | Control::Toggle(_)
                    | Control::Dropdown(_)
                    | Control::Stepper(_)
                    | Control::Separator => None,
                })
            }
            DropdownSection::Options { .. } => None,
        })
        .unwrap_or((Palette::Soft, PaintRole::Fill))
}

/// Whether `dropdown` opens a list of presets, whose trigger is `px-2` with
/// a 10 px chevron (`sizeTriggerClass` in `PagePopup.tsx`).
fn is_preset(dropdown: &Dropdown) -> bool {
    dropdown.content.iter().any(|section| match section {
        DropdownSection::Options { options, .. } => {
            options.iter().any(|option| option.trailing.is_some())
        }
        DropdownSection::Controls(_) => false,
    })
}

/// The chevron's side.
fn chevron_size(dropdown: &Dropdown) -> f32 {
    if is_preset(dropdown) {
        PRESET_CHEVRON
    } else {
        CHEVRON
    }
}

/// What a closed dropdown shows before its chevron: the padding either side
/// (`pl-*`/`pr-*` of each `*Dropdown.tsx` trigger) and the width between.
fn summary_metrics(ctx: &Ctx<'_>, dropdown: &Dropdown) -> (f32, f32, f32) {
    let face = &dropdown.summary;
    match (face.icon, &face.text) {
        (_, Some(label)) => {
            let font = face.font.unwrap_or(TextFont::Sans);
            let pad = if is_preset(dropdown) { 8.0 } else { 6.0 };
            (pad, ctx.text_width(label, font), pad)
        }
        (Some(Icon::Shape(_)), None) => (6.0, TRIGGER_SHAPE, 6.0),
        (Some(Icon::Ban) | None, None) => (4.0, TRIGGER_DOT, 6.0),
        (Some(_), None) => (6.0, ICON, 4.0),
    }
}

/// The width of the closed dropdown.
pub(super) fn width(ctx: &Ctx<'_>, dropdown: &Dropdown) -> f32 {
    let (before, summary, after) = summary_metrics(ctx, dropdown);
    before + summary + GAP + chevron_size(dropdown) + after
}

/// The closed dropdown as a control `left` along its row: what it shows
/// now, and a chevron.
pub(super) fn node(ctx: &Ctx<'_>, dropdown: &Dropdown, left: f32) -> Node {
    let (before, summary, after) = summary_metrics(ctx, dropdown);
    let chevron = chevron_size(dropdown);
    let rect = PanelRect::new(left, 0.0, before + summary + GAP + chevron + after, CONTROL);
    let slot = PanelRect::new(left + before, 0.0, summary, CONTROL);
    let face = &dropdown.summary;
    let (palette, role) = swatch_style(dropdown);
    let dot = |tint| Part::Dot {
        rect: slot.centred(Vec2::splat(TRIGGER_DOT)),
        tint,
    };
    let mut parts = match (face.icon, &face.text) {
        (_, Some(label)) => vec![text(
            label.clone(),
            slot,
            TextAlign::Left,
            face.font,
            Tone::Follow,
        )],
        (Some(Icon::Ban), None) => vec![
            dot(None),
            Part::Icon {
                icon: Icon::Ban,
                rect: slot.centred(Vec2::splat(BAN)),
                tint: None,
            },
        ],
        (Some(icon), None) => vec![Part::Icon {
            icon,
            rect: slot.centred(Vec2::splat(summary)),
            tint: None,
        }],
        (None, None) => vec![dot(face.color.clone().map(|color| Tint {
            color,
            palette,
            role,
        }))],
    };
    parts.push(Part::Chevron {
        rect: PanelRect::new(
            slot.right() + GAP,
            (CONTROL - chevron) / 2.0,
            chevron,
            chevron,
        ),
    });
    let open = ctx.ui.open.as_ref() == Some(&dropdown.id);
    Node {
        parts,
        run: Some(Run::Toggle),
        ..pressable(ctx, &dropdown.id, rect, true, open)
    }
}
