//! The dock's controls for an edge (`EdgePopup.tsx`): color, stroke, arrowheads and
//! delete.

use specular_doc::{EdgeEnd, LineStyle};

use super::super::build::{button, color_dropdown, groups, toggle};
use super::super::{
    Control, ControlId, ControlsModel, Dropdown, DropdownSection, Face, Field, FieldSubmit,
    FieldWidth, Icon, PaintRole, Palette,
};
use crate::property::read;
use crate::{Action, App, Property};

/// The thin and thick widths an edge is offered in.
const WIDTHS: [(f64, &str, &str, Icon); 2] = [
    (1.5, "thin", "Thin edge", Icon::StrokeThin),
    (3.0, "thick", "Thick edge", Icon::StrokeThick),
];

fn stroke_dropdown(app: &App) -> Control {
    let id = ControlId::new("edge.stroke");
    let width = read::stroke_width(app);
    let style = read::line_style(app);
    let mut row: Vec<Control> = (WIDTHS.into_iter())
        .map(|(value, name, label, icon)| {
            Control::Toggle(toggle(
                id.child(name),
                label,
                Face::icon(icon),
                width == Some(value),
                Action::SetProperty(Property::StrokeWidth(value)),
            ))
        })
        .collect();
    row.push(Control::Separator);
    let styles = [
        (LineStyle::Solid, "solid", "Regular edge", Icon::LineSolid),
        (LineStyle::Dashed, "dashed", "Dashed edge", Icon::LineDashed),
    ];
    row.extend(styles.into_iter().map(|(value, name, label, icon)| {
        Control::Toggle(toggle(
            id.child(name),
            label,
            Face::icon(icon),
            style == Some(value),
            Action::SetProperty(Property::LineStyle(value)),
        ))
    }));
    Control::Dropdown(Dropdown {
        label: "Edge stroke".into(),
        summary: Face::icon(Icon::Border),
        content: vec![DropdownSection::Controls(row)],
        id,
    })
}

/// The arrowhead toggles: each goes to the opposite of what it is now.
fn arrowheads(app: &App) -> Vec<Control> {
    let flipped = |on: bool| if on { EdgeEnd::None } else { EdgeEnd::Arrow };
    let start = read::from_end(app) == Some(EdgeEnd::Arrow);
    let end = read::to_end(app) == Some(EdgeEnd::Arrow);
    vec![
        Control::Toggle(toggle(
            ControlId::new("edge.start"),
            "Toggle start arrowhead",
            Face::icon(Icon::ArrowStart),
            start,
            Action::SetProperty(Property::FromEnd(flipped(start))),
        )),
        Control::Toggle(toggle(
            ControlId::new("edge.end"),
            "Toggle end arrowhead",
            Face::icon(Icon::ArrowEnd),
            end,
            Action::SetProperty(Property::ToEnd(flipped(end))),
        )),
    ]
}

/// The label to type, as `Label…` in the Electron popup.
fn label(app: &App) -> Control {
    Control::Field(Field {
        id: ControlId::new("edge.label"),
        label: "Edge label".into(),
        caption: None,
        value: read::edge_label(app).unwrap_or_default(),
        placeholder: Some("Label\u{2026}".into()),
        width: FieldWidth::Medium,
        submit: FieldSubmit::EdgeLabel,
    })
}

pub(super) fn controls(app: &App) -> ControlsModel {
    let color = read::color(app);
    let controls = groups(vec![
        vec![color_dropdown(
            ControlId::new("edge.color"),
            "edge",
            Palette::Vivid,
            PaintRole::Ink,
            color.as_ref(),
            None,
            |color| Action::SetProperty(Property::Color(color)),
        )],
        vec![stroke_dropdown(app)],
        arrowheads(app),
        vec![label(app)],
        vec![button(
            ControlId::new("edge.delete"),
            "Delete edge",
            Face::icon(Icon::Trash),
            Action::Delete,
        )],
    ]);
    ControlsModel { controls }
}
