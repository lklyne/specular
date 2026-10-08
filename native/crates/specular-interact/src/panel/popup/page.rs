//! The popup of pages (`PagePopup.tsx`): size, device frame, orientation and
//! color scheme. Navigation, the address, syncing, the repo binding and
//! focus have no action here.

use specular_doc::{ColorScheme, Entity, VIEWPORT_PRESETS};

use super::super::build::{button, groups, toggle};
use super::super::{
    Align, Control, ControlId, Dropdown, DropdownOption, DropdownSection, Face, Icon, OptionLayout,
    PopupModel,
};
use crate::property::read;
use crate::{Action, App, Orientation, Property};

/// The preset rows in the order the size list has them: phones, tablets,
/// then laptops and desktops.
const SECTIONS: [&[u32]; 3] = [&[0, 1, 2, 9, 10], &[3, 4, 5], &[6, 7, 8]];

/// A phone's row drops the model year: the size says which it is.
fn row_label(label: &str) -> String {
    label.replace("iPhone 14 ", "iPhone ")
}

fn size_dropdown(app: &App, single: bool) -> Control {
    let id = ControlId::new("page.size");
    let preset = read::viewport_preset(app);
    let mut sections: Vec<DropdownSection> = SECTIONS
        .iter()
        .map(|rows| DropdownSection::Options {
            layout: OptionLayout::List,
            options: (rows.iter())
                .filter_map(|&index| {
                    let row = VIEWPORT_PRESETS.get(usize::try_from(index).ok()?)?;
                    Some(DropdownOption {
                        id: id.child(index),
                        label: row.label.into(),
                        face: Face::text(row_label(row.label)),
                        trailing: Some(format!("{}\u{d7}{}", row.width, row.height).into()),
                        chord: None,
                        selected: preset == Some(index),
                        action: Action::SetProperty(Property::ViewportPreset(index)),
                    })
                })
                .collect(),
        })
        .collect();
    // Several pages have no one custom size to keep.
    if single {
        sections.push(DropdownSection::Options {
            layout: OptionLayout::List,
            options: vec![DropdownOption {
                id: id.child("custom"),
                label: "Custom".into(),
                face: Face::text("Custom"),
                trailing: None,
                chord: None,
                selected: preset.is_none(),
                action: Action::SetProperty(Property::CustomViewport),
            }],
        });
    }
    let named = preset.and_then(|index| VIEWPORT_PRESETS.get(usize::try_from(index).ok()?));
    let summary = match (named, single) {
        (Some(row), _) => row.label,
        (None, true) => "Custom",
        (None, false) => "Multiple",
    };
    Control::Dropdown(Dropdown {
        label: "Page size".into(),
        summary: Face::text(summary),
        content: sections,
        id,
    })
}

/// The frame toggle goes to on, unless every page has it.
fn frame_toggle(app: &App, label: &'static str) -> Control {
    let on = read::device_frame(app) == Some(true);
    Control::Toggle(toggle(
        ControlId::new("page.frame"),
        label,
        Face::icon(Icon::Device),
        on,
        Action::SetProperty(Property::DeviceFrame(!on)),
    ))
}

fn rotate(app: &App) -> Control {
    let next = match read::orientation(app) {
        Some(Orientation::Landscape) => Orientation::Portrait,
        Some(Orientation::Portrait) | None => Orientation::Landscape,
    };
    button(
        ControlId::new("page.rotate"),
        "Rotate viewport",
        Face::icon(Icon::Rotate),
        Action::SetProperty(Property::Orientation(next)),
    )
}

/// The color scheme button steps through system, light and dark.
fn scheme(app: &App) -> Control {
    let (name, icon, next) = match read::color_scheme(app) {
        Some(Some(ColorScheme::Light)) => ("Light", Icon::SchemeLight, Some(ColorScheme::Dark)),
        Some(Some(ColorScheme::Dark)) => ("Dark", Icon::SchemeDark, None),
        Some(None) | None => ("System", Icon::SchemeSystem, Some(ColorScheme::Light)),
    };
    button(
        ControlId::new("page.scheme"),
        format!("Color scheme: {name}. Click to change."),
        Face::icon(icon),
        Action::SetProperty(Property::ColorScheme(next)),
    )
}

pub(super) fn popup(app: &App, entities: &[&Entity]) -> PopupModel {
    let single = entities.len() == 1;
    let controls = if single {
        groups(vec![
            vec![size_dropdown(app, true)],
            vec![frame_toggle(app, "Device frame"), rotate(app)],
            vec![scheme(app)],
        ])
    } else {
        groups(vec![
            vec![size_dropdown(app, false)],
            vec![frame_toggle(app, "Toggle device frame for selected pages")],
        ])
    };
    let align = if single {
        Align::Stretch
    } else {
        Align::Center
    };
    PopupModel {
        anchor: super::over_titled(entities, align),
        controls,
    }
}
