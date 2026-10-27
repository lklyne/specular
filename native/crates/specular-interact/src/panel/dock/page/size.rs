//! The size list: presets, the size fields, the device frame, orientation and color scheme.

use specular_doc::{
    ColorScheme, DeviceFrameColor, DeviceFrameStyle, Entity, PRESET_MAIN, PRESET_MENU,
    VIEWPORT_PRESETS,
};

use super::super::super::{
    Control, ControlId, Dropdown, DropdownOption, DropdownSection, Face, Field, FieldSubmit,
    FieldWidth, Icon, OptionLayout,
};
use crate::property::read;
use crate::{Action, App, Orientation, Property};

/// The rows of one section of the preset list.
fn preset_rows(
    id: &ControlId,
    rows: &[u32],
    selected: Option<u32>,
    verb: Option<&str>,
    pick: &impl Fn(u32) -> Action,
) -> Vec<DropdownOption> {
    (rows.iter())
        .filter_map(|&index| {
            let row = VIEWPORT_PRESETS.get(usize::try_from(index).ok()?)?;
            Some(DropdownOption {
                id: id.child(index),
                label: match verb {
                    Some(verb) => format!("{verb} {}", row.label).into(),
                    None => row.label.into(),
                },
                face: Face::text(row.label),
                trailing: Some(format!("{}\u{d7}{}", row.width, row.height).into()),
                fields: Vec::new(),
                chord: None,
                selected: selected == Some(index),
                enabled: true,
                action: pick(index),
            })
        })
        .collect()
}

/// The sections of [`PRESET_MENU`] holding the rows `keep` lets through. A
/// section left with no rows is dropped.
fn preset_groups(
    id: &ControlId,
    selected: Option<u32>,
    verb: Option<&str>,
    pick: &impl Fn(u32) -> Action,
    keep: impl Fn(u32) -> bool,
) -> Vec<DropdownSection> {
    (PRESET_MENU.iter())
        .filter_map(|(_, rows)| {
            let rows: Vec<u32> = rows.iter().copied().filter(|&index| keep(index)).collect();
            (!rows.is_empty()).then(|| DropdownSection::Options {
                layout: OptionLayout::List,
                options: preset_rows(id, &rows, selected, verb, pick),
            })
        })
        .collect()
}

/// The preset list shared by the page size dropdown and the page tool:
/// the sizes of [`PRESET_MAIN`] in a section each for phones, tablets and
/// computers, then a "More" row that opens the rest of [`PRESET_MENU`], all
/// named from `id`.
/// `selected` is the preset in use. `custom` adds a row over "More" that is
/// on when its flag is, runs its action, and has its fields at its far end.
/// `verb` starts each row's accessible name ("Add iPhone SE").
pub(crate) fn preset_sections(
    id: &ControlId,
    selected: Option<u32>,
    verb: Option<&str>,
    pick: impl Fn(u32) -> Action,
    custom: Option<(bool, Action, Vec<Field>)>,
) -> Vec<DropdownSection> {
    let main = |index: u32| PRESET_MAIN.contains(&index);
    let mut sections = preset_groups(id, selected, verb, &pick, main);
    if let Some((selected, action, fields)) = custom {
        sections.push(DropdownSection::Options {
            layout: OptionLayout::List,
            options: vec![DropdownOption {
                id: id.child("custom"),
                label: match verb {
                    Some(verb) => format!("{verb} custom").into(),
                    None => "Custom".into(),
                },
                face: Face::text("Custom"),
                trailing: None,
                fields,
                chord: None,
                selected,
                enabled: true,
                action,
            }],
        });
    }
    sections.push(DropdownSection::Controls(vec![Control::Dropdown(
        Dropdown {
            id: id.child("more"),
            label: "More sizes".into(),
            summary: Face::text("More"),
            content: preset_groups(id, selected, verb, &pick, |index| !main(index)),
        },
    )]));
    sections
}

/// A page's size as a field shows it.
fn pixels(size: f64) -> String {
    format!("{size:.0}")
}

/// The width and height of the one page selected, to type a custom size in.
fn size_fields(id: &ControlId, page: &Entity) -> Vec<Field> {
    let field = |name: &'static str, caption: &'static str, value, submit| Field {
        id: id.child(name),
        label: format!("Page {name}").into(),
        caption: Some(caption.into()),
        value: pixels(value),
        placeholder: None,
        width: FieldWidth::Short,
        submit,
    };
    vec![
        field("width", "W", page.rect.width, FieldSubmit::ViewportWidth),
        field("height", "H", page.rect.height, FieldSubmit::ViewportHeight),
    ]
}

pub(super) fn size_dropdown(app: &App, single: Option<&Entity>) -> Control {
    let id = ControlId::new("page.size");
    let preset = read::viewport_preset(app);
    // Several pages have no one custom size to keep.
    let custom = single.map(|page| {
        (
            preset.is_none(),
            Action::SetProperty(Property::CustomViewport),
            size_fields(&id, page),
        )
    });
    let mut sections = preset_sections(
        &id,
        preset,
        None,
        |index| Action::SetProperty(Property::ViewportPreset(index)),
        custom,
    );
    // How the pages are shown, under the sizes. Several pages share only
    // the frame, and a device has a colour only while it is drawn.
    sections.push(frame_choice(app, "page.frame"));
    sections
        .extend(read::any_device_frame(app).then(|| frame_color_choice(app, "page.frame-color")));
    if single.is_some() {
        sections.extend([orientation_choice(app), scheme_choice(app)]);
    }
    let named = preset.and_then(|index| VIEWPORT_PRESETS.get(usize::try_from(index).ok()?));
    let summary = match (named, single.is_some()) {
        (Some(row), _) => row.label,
        (None, true) => "Custom",
        (None, false) => "Multiple",
    };
    Control::Dropdown(Dropdown {
        label: "Page size".into(),
        summary: Face {
            icon: Some(Icon::Proportions),
            ..Face::text(summary)
        },
        content: sections,
        id,
    })
}

/// One line of the size list: `title` and the values it can be set to,
/// each named `id.<name>` and on while `current` is its value.
fn choice<T: PartialEq + Copy>(
    id: &'static str,
    title: &'static str,
    current: Option<T>,
    values: &[(&'static str, &'static str, T)],
    set: impl Fn(T) -> Property,
) -> DropdownSection {
    let id = ControlId::new(id);
    DropdownSection::Options {
        layout: OptionLayout::Segmented { title },
        options: (values.iter())
            .map(|&(name, word, value)| DropdownOption {
                id: id.child(name),
                label: format!("{title}: {word}").into(),
                face: Face::text(word),
                trailing: None,
                fields: Vec::new(),
                chord: None,
                selected: current == Some(value),
                enabled: true,
                action: Action::SetProperty(set(value)),
            })
            .collect(),
    }
}

/// The device frame the selection is drawn in, named `id`.
pub(in super::super) fn frame_choice(app: &App, id: &'static str) -> DropdownSection {
    choice(
        id,
        "Frame",
        read::device_frame(app),
        &[
            ("none", "None", None),
            ("simple", "Simple", Some(DeviceFrameStyle::Clay)),
            ("realistic", "Realistic", Some(DeviceFrameStyle::Realistic)),
        ],
        Property::DeviceFrame,
    )
}

/// The colour the devices are drawn in, named `id`.
pub(in super::super) fn frame_color_choice(app: &App, id: &'static str) -> DropdownSection {
    choice(
        id,
        "Color",
        read::device_frame_color(app),
        &[
            ("light", "Light", DeviceFrameColor::Light),
            ("dark", "Dark", DeviceFrameColor::Dark),
        ],
        Property::DeviceFrameColor,
    )
}

/// Which way up the page is.
fn orientation_choice(app: &App) -> DropdownSection {
    choice(
        "page.rotate",
        "Orientation",
        read::orientation(app),
        &[
            ("portrait", "Portrait", Orientation::Portrait),
            ("landscape", "Landscape", Orientation::Landscape),
        ],
        Property::Orientation,
    )
}

/// The color scheme the page is shown in: its own, or the app's while it
/// has none. Choosing the app's leaves the page following it again.
fn scheme_choice(app: &App) -> DropdownSection {
    let followed = read::followed_color_scheme(app);
    choice(
        "page.scheme",
        "Theme",
        read::color_scheme(app).map(|own| own.unwrap_or(followed)),
        &[
            ("light", "Light", ColorScheme::Light),
            ("dark", "Dark", ColorScheme::Dark),
        ],
        |scheme| Property::ColorScheme((scheme != followed).then_some(scheme)),
    )
}
