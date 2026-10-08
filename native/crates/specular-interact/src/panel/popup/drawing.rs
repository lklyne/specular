//! The popup of drawings (`DrawingPopup.tsx`): brush, width, color. The
//! draw tool's popup shares the brush and width rows.

use specular_doc::{BrushType, Color, Entity};

use super::super::build::{color_dropdown, groups, noun, toggle};
use super::super::{Align, Control, ControlId, Face, Icon, PaintRole, Palette, PopupModel};
use super::actions::Actions;
use crate::property::read;
use crate::tool_defaults::{nearest_width, width_presets};
use crate::{Action, Property};

/// The palette a brush paints in: the highlighter in pastels, the pen in
/// saturated hues.
pub(super) const fn palette_of(brush: BrushType) -> Palette {
    match brush {
        BrushType::Pen => Palette::Vivid,
        BrushType::Highlight => Palette::Soft,
    }
}

/// The two brushes, the glyph tinted with the stroke color. `current` is
/// the brush in use, if all agree.
pub(super) fn brushes(
    ink: Option<&Color>,
    current: Option<BrushType>,
    set: impl Fn(BrushType) -> Action,
) -> Vec<Control> {
    let all = [
        (BrushType::Pen, "pen", "Pen", Icon::BrushPen),
        (
            BrushType::Highlight,
            "highlighter",
            "Highlighter",
            Icon::BrushHighlighter,
        ),
    ];
    (all.into_iter())
        .map(|(brush, name, label, icon)| {
            Control::Toggle(toggle(
                ControlId::new("brush").child(name),
                label,
                Face::icon(icon).tinted(ink.cloned()),
                current == Some(brush),
                set(brush),
            ))
        })
        .collect()
}

/// The thin and thick widths of `brush`; the one nearest `width` is on.
pub(super) fn widths(
    brush: BrushType,
    width: Option<f64>,
    set: impl Fn(f64) -> Action,
) -> Vec<Control> {
    let nearest = width.map(|width| nearest_width(brush, width));
    let [thin, thick] = width_presets(brush);
    [
        (thin, "thin", Icon::StrokeThin),
        (thick, "thick", Icon::StrokeThick),
    ]
    .into_iter()
    .map(|(value, name, icon)| {
        Control::Toggle(toggle(
            ControlId::new("width").child(name),
            format!("Set width to {value}px"),
            Face::icon(icon),
            nearest == Some(value),
            set(value),
        ))
    })
    .collect()
}

pub(super) fn popup(app: &crate::App, entities: &[&Entity]) -> PopupModel {
    let noun = noun(entities.len(), "drawing", "drawings");
    let brush = read::brush(app);
    let color = read::color(app);
    let controls = groups(vec![
        brushes(color.as_ref(), brush, |brush| {
            Action::SetProperty(Property::Brush(brush))
        }),
        widths(
            brush.unwrap_or(BrushType::Pen),
            read::stroke_width(app),
            |width| Action::SetProperty(Property::StrokeWidth(width)),
        ),
        vec![color_dropdown(
            ControlId::new("drawing.color"),
            &noun,
            palette_of(brush.unwrap_or(BrushType::Pen)),
            PaintRole::Ink,
            color.as_ref(),
            None,
            |color| Action::SetProperty(Property::Color(color)),
        )],
        Actions::all(&noun, entities.len()).controls(),
    ]);
    PopupModel {
        anchor: super::over(entities, Align::Center),
        controls,
    }
}
