//! The popup of shapes (`ShapePopup.tsx`), and the shape-kind dropdown the
//! shape tool's popup shares.

use specular_doc::{BorderStyle, Entity, FillStyle, ShapeKind, TextAlign};

use super::super::build::{color_dropdown, groups, noun, size_dropdown, swatches, toggle};
use super::super::{
    Align, Control, ControlId, Dropdown, DropdownOption, DropdownSection, Face, Icon, OptionLayout,
    PaintRole, Palette, PopupModel,
};
use super::actions::Actions;
use crate::property::read;
use crate::{Action, Property};

/// Every silhouette in dropdown order, with its name.
const KINDS: [(ShapeKind, &str, &str); 10] = [
    (ShapeKind::Rectangle, "rectangle", "Rectangle"),
    (ShapeKind::Rounded, "rounded", "Rounded rectangle"),
    (ShapeKind::Ellipse, "ellipse", "Ellipse"),
    (ShapeKind::Diamond, "diamond", "Diamond"),
    (ShapeKind::Triangle, "triangle", "Triangle"),
    (ShapeKind::Hexagon, "hexagon", "Hexagon"),
    (ShapeKind::Pill, "pill", "Pill"),
    (ShapeKind::Parallelogram, "parallelogram", "Parallelogram"),
    (ShapeKind::Chevron, "chevron", "Chevron"),
    (ShapeKind::Cylinder, "cylinder", "Cylinder"),
];

/// The border thicknesses on offer.
const BORDER_WIDTHS: [f64; 4] = [1.0, 2.0, 3.0, 4.0];

/// A grid of every silhouette; closed it shows the current one.
pub(super) fn kind_dropdown(
    id: ControlId,
    label: &'static str,
    selected: Option<ShapeKind>,
    set: impl Fn(ShapeKind) -> Action,
) -> Control {
    let options = (KINDS.into_iter())
        .map(|(kind, name, label)| DropdownOption {
            id: id.child(name),
            label: label.into(),
            face: Face::icon(Icon::Shape(kind)),
            trailing: None,
            chord: None,
            selected: selected == Some(kind),
            enabled: true,
            action: set(kind),
        })
        .collect();
    Control::Dropdown(Dropdown {
        label: label.into(),
        summary: Face::icon(Icon::Shape(selected.unwrap_or(ShapeKind::Rectangle))),
        content: vec![DropdownSection::Options {
            layout: OptionLayout::Grid { columns: 5 },
            options,
        }],
        id,
    })
}

fn align_dropdown(app: &crate::App) -> Control {
    let id = ControlId::new("shape.align");
    let current = read::text_align(app);
    let all = [
        (TextAlign::Left, "left", "Align text left", Icon::AlignLeft),
        (
            TextAlign::Center,
            "center",
            "Align text center",
            Icon::AlignCenter,
        ),
        (
            TextAlign::Right,
            "right",
            "Align text right",
            Icon::AlignRight,
        ),
    ];
    let shown = (all.iter())
        .find(|(align, ..)| Some(*align) == current)
        .map_or(Icon::AlignCenter, |(.., icon)| *icon);
    let options = (all.into_iter())
        .map(|(align, name, label, icon)| DropdownOption {
            id: id.child(name),
            label: label.into(),
            face: Face::icon(icon),
            trailing: None,
            chord: None,
            selected: Some(align) == current,
            enabled: true,
            action: Action::SetProperty(Property::TextAlign(align)),
        })
        .collect();
    Control::Dropdown(Dropdown {
        label: "Text alignment".into(),
        summary: Face::icon(shown),
        content: vec![DropdownSection::Options {
            layout: OptionLayout::Row,
            options,
        }],
        id,
    })
}

/// Line style and thickness in one row, border color under it. With no
/// border, thickness and color have nothing to set.
fn border_dropdown(app: &crate::App) -> Control {
    let id = ControlId::new("shape.border");
    let style = read::border_style(app);
    let width = read::stroke_width(app);
    let bordered = style != Some(BorderStyle::None);
    let styles = [
        (BorderStyle::Solid, "solid", "Solid", Icon::LineSolid),
        (BorderStyle::Dashed, "dashed", "Dashed", Icon::LineDashed),
        (BorderStyle::None, "none", "None", Icon::Ban),
    ];
    let mut row: Vec<Control> = (styles.into_iter())
        .map(|(value, name, label, icon)| {
            let mut face = Face::icon(icon);
            face.text = Some(label.into());
            Control::Toggle(toggle(
                id.child(name),
                label,
                face,
                style == Some(value),
                Action::SetProperty(Property::BorderStyle(value)),
            ))
        })
        .collect();
    row.extend(BORDER_WIDTHS.into_iter().map(|value| {
        let mut thickness = toggle(
            id.child(format!("w{value}")),
            format!("Set border width to {value}px"),
            Face::text(format!("{value}")),
            bordered && width == Some(value),
            Action::SetProperty(Property::StrokeWidth(value)),
        );
        thickness.enabled = bordered;
        Control::Toggle(thickness)
    }));
    let color = read::border_color(app);
    let mut colors = swatches(
        id.child("color"),
        Palette::Soft,
        PaintRole::Fill,
        color.as_ref(),
        None,
        |color| Action::SetProperty(Property::BorderColor(color)),
    );
    colors.enabled = bordered;
    Control::Dropdown(Dropdown {
        label: "Border".into(),
        summary: Face::icon(Icon::Border),
        content: vec![
            DropdownSection::Controls(row),
            DropdownSection::Controls(vec![Control::Swatches(colors)]),
        ],
        id,
    })
}

pub(super) fn popup(app: &crate::App, entities: &[&Entity]) -> PopupModel {
    let noun = noun(entities.len(), "shape", "shapes");
    let color = read::color(app);
    let clear = (
        read::fill_style(app) == Some(FillStyle::None),
        Action::SetProperty(Property::FillStyle(FillStyle::None)),
    );
    let controls = groups(vec![
        vec![kind_dropdown(
            ControlId::new("shape.kind"),
            "Set shape",
            read::shape_kind(app),
            |kind| Action::SetProperty(Property::ShapeKind(kind)),
        )],
        vec![size_dropdown(
            ControlId::new("shape.size"),
            "Set label size",
            read::text_size(app),
            |size| Action::SetProperty(Property::TextSize(size)),
        )],
        vec![align_dropdown(app)],
        vec![color_dropdown(
            ControlId::new("shape.color"),
            &noun,
            Palette::Soft,
            PaintRole::Fill,
            color.as_ref(),
            Some(clear),
            |color| Action::SetProperty(Property::Color(color)),
        )],
        vec![border_dropdown(app)],
        Actions::all(&noun, entities.len()).controls(),
    ]);
    PopupModel {
        anchor: super::over(entities, Align::Center),
        controls,
    }
}
