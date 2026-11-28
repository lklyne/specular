//! The dock's controls for shapes (`ShapePopup.tsx`), and the shape-kind dropdown the
//! shape tool's options share.

use specular_doc::{BorderStyle, CustomShape, Entity, FillStyle, ShapeKind, TextAlign};

use super::super::build::{color_dropdown, groups, noun, size_dropdown, swatches, toggle};
use super::super::{
    Control, ControlId, ControlsModel, Dropdown, DropdownOption, DropdownSection, Face, Icon,
    OptionLayout, PaintRole, Palette,
};
use super::actions::Actions;
use crate::mods::ModToolId;
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

/// What a shape-kind list picks: one of the app's silhouettes or a mod's.
pub(super) enum Pick {
    /// One of the app's.
    Kind(ShapeKind),
    /// A mod's, as its `mod.json` gives a new one.
    Custom(CustomShape),
}

/// A grid of every silhouette, the app's and then each installed mod's
/// shapes; closed it shows the current one. `custom` names the mod shape
/// that is current, when one is.
pub(super) fn kind_dropdown(
    app: &crate::App,
    id: ControlId,
    label: &'static str,
    selected: Option<ShapeKind>,
    custom: Option<&str>,
    set: impl Fn(Pick) -> Action,
) -> Control {
    let builtin = (KINDS.into_iter()).map(|(kind, name, label)| DropdownOption {
        id: id.child(name),
        label: label.into(),
        face: Face::icon(Icon::Shape(kind)),
        trailing: None,
        fields: Vec::new(),
        chord: None,
        selected: selected == Some(kind),
        enabled: true,
        action: set(Pick::Kind(kind)),
    });
    let catalog = app.mod_catalog();
    let mods = (catalog.shapes()).filter_map(|(installed, spec)| {
        let shape = spec.custom(&installed.name).ok()?;
        Some(DropdownOption {
            id: id.child(&shape.name),
            label: spec.title.clone().into(),
            face: Face::icon(Icon::named_or(
                spec.icon.as_deref(),
                Icon::Shape(ShapeKind::Custom),
            )),
            trailing: None,
            fields: Vec::new(),
            chord: None,
            selected: custom == Some(shape.name.as_str()),
            enabled: true,
            action: set(Pick::Custom(shape)),
        })
    });
    let options: Vec<DropdownOption> = builtin.chain(mods).collect();
    let summary = (options.iter().find(|option| option.selected)).map_or_else(
        || Face::icon(Icon::Shape(selected.unwrap_or(ShapeKind::Rectangle))),
        |option| option.face.clone(),
    );
    Control::Dropdown(Dropdown {
        label: label.into(),
        summary,
        content: vec![DropdownSection::Options {
            layout: OptionLayout::Grid { columns: 5 },
            options,
        }],
        id,
    })
}

/// The shape tool's dropdown: every silhouette, then every installed mod
/// tool that sits among the shapes (a tool with `"group": "shape"`).
/// `armed` is the mod tool in hand, which a silhouette's choice puts down
/// for the shape tool.
pub(super) fn tool_kind_dropdown(app: &crate::App, armed: Option<ModToolId>) -> Control {
    let defaults = &app.tool_defaults().shape;
    let set = |pick| {
        let patch = match pick {
            Pick::Kind(kind) => crate::ToolDefaultPatch::ShapeKind(kind),
            Pick::Custom(shape) => crate::ToolDefaultPatch::CustomShape(Box::new(shape)),
        };
        match armed {
            Some(_) => Action::SetToolVariant(patch),
            None => Action::SetToolDefault(patch),
        }
    };
    let id = ControlId::new("shape.kind");
    let custom = (defaults.custom.as_ref()).map(|custom| custom.name.as_str());
    let mut dropdown = kind_dropdown(
        app,
        id.clone(),
        "Set default shape",
        armed.is_none().then_some(defaults.kind),
        custom.filter(|_| armed.is_none()),
        set,
    );
    let catalog = app.mod_catalog();
    let tools: Vec<DropdownOption> = (catalog.tools())
        .filter(|(_, _, spec)| spec.is_shape())
        .map(|(tool, installed, spec)| {
            let word = format!("{}/{}", installed.name, spec.name);
            DropdownOption {
                id: id.child(&word),
                label: spec.title.clone().into(),
                face: Face::icon(Icon::named_or(spec.icon.as_deref(), Icon::Code)),
                trailing: None,
                fields: Vec::new(),
                chord: None,
                selected: armed == Some(tool),
                enabled: true,
                action: Action::Mod(Box::new(crate::mods::ModAction::SetTool(word))),
            }
        })
        .collect();
    if let Control::Dropdown(Dropdown {
        content, summary, ..
    }) = &mut dropdown
    {
        if let Some(chosen) = tools.iter().find(|option| option.selected) {
            *summary = chosen.face.clone();
        }
        if let Some(DropdownSection::Options { options, .. }) = content.first_mut() {
            options.extend(tools);
        }
    }
    dropdown
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
            fields: Vec::new(),
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

pub(super) fn controls(app: &crate::App, entities: &[&Entity]) -> ControlsModel {
    let noun = noun(entities.len(), "shape", "shapes");
    let color = read::color(app);
    let clear = (
        read::fill_style(app) == Some(FillStyle::None),
        Action::SetProperty(Property::FillStyle(FillStyle::None)),
    );
    let controls = groups(vec![
        vec![kind_dropdown(
            app,
            ControlId::new("shape.kind"),
            "Set shape",
            read::shape_kind(app),
            read::custom_shape(app).as_deref(),
            |pick| match pick {
                Pick::Kind(kind) => Action::SetProperty(Property::ShapeKind(kind)),
                Pick::Custom(shape) => Action::SetProperty(Property::CustomShape(Box::new(shape))),
            },
        )],
        super::modded::shape_buttons(app, entities),
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
    ControlsModel { controls }
}
