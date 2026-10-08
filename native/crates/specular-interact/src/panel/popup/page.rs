//! The popup of pages (`PagePopup.tsx`): history, reload and the address,
//! then size, device frame, orientation and color scheme, and the chain
//! button that syncs the selected pages, and the repo control of a single
//! page. Electron also infers a repo from a running dev server's base URL;
//! this app runs no dev servers, so the origin's binding is the only source.

use specular_agent::origin_of;
use specular_doc::{ColorScheme, Entity, Kind, VIEWPORT_PRESETS};

use super::super::build::{button, groups, toggle};
use super::super::{
    Align, Button, Control, ControlId, Dropdown, DropdownOption, DropdownSection, Face, Field,
    FieldSubmit, FieldWidth, Icon, OptionLayout, PopupModel,
};
use super::actions::Actions;
use crate::property::read;
use crate::{Action, App, Orientation, Property, RepoAction, binding_of};

/// The preset rows in the order the size list has them: phones, tablets,
/// then laptops and desktops.
const SECTIONS: [&[u32]; 3] = [&[0, 1, 2, 9, 10], &[3, 4, 5], &[6, 7, 8]];

/// A phone's row drops the model year: the size says which it is.
fn row_label(label: &str) -> String {
    label.replace("iPhone 14 ", "iPhone ")
}

/// The preset list shared by the page size dropdown and the page tool:
/// a section of rows each for phones, tablets and desktops, named from `id`.
/// `selected` is the preset in use. `custom` adds a last row that is on when
/// its flag is, and runs its action. `verb` starts each row's accessible
/// name ("Add iPhone SE").
pub(super) fn preset_sections(
    id: &ControlId,
    selected: Option<u32>,
    verb: Option<&str>,
    pick: impl Fn(u32) -> Action,
    custom: Option<(bool, Action)>,
) -> Vec<DropdownSection> {
    let named = |name: &str| -> super::super::Label {
        match verb {
            Some(verb) => format!("{verb} {name}").into(),
            None => name.to_owned().into(),
        }
    };
    let mut sections: Vec<DropdownSection> = SECTIONS
        .iter()
        .map(|rows| DropdownSection::Options {
            layout: OptionLayout::List,
            options: (rows.iter())
                .filter_map(|&index| {
                    let row = VIEWPORT_PRESETS.get(usize::try_from(index).ok()?)?;
                    Some(DropdownOption {
                        id: id.child(index),
                        label: named(row.label),
                        face: Face::text(row_label(row.label)),
                        trailing: Some(format!("{}\u{d7}{}", row.width, row.height).into()),
                        chord: None,
                        selected: selected == Some(index),
                        enabled: true,
                        action: pick(index),
                    })
                })
                .collect(),
        })
        .collect();
    if let Some((selected, action)) = custom {
        sections.push(DropdownSection::Options {
            layout: OptionLayout::List,
            options: vec![DropdownOption {
                id: id.child("custom"),
                label: named(if verb.is_some() { "custom" } else { "Custom" }),
                face: Face::text("Custom"),
                trailing: None,
                chord: None,
                selected,
                enabled: true,
                action,
            }],
        });
    }
    sections
}

/// A button of the history row. It cannot be pressed where the page has said
/// there is nowhere to go.
fn history_button(
    id: &'static str,
    label: &'static str,
    icon: Icon,
    action: Action,
    enabled: bool,
) -> Control {
    Control::Button(Button {
        id: ControlId::new(id),
        label: label.into(),
        face: Face::icon(icon),
        enabled,
        chord: binding_of(&action).map(|binding| binding.chord),
        action,
    })
}

/// Back, forward, and reload, which is stop while the page loads.
fn history(app: &App, page: &Entity) -> Vec<Control> {
    let state = app.page_state(&page.id);
    let can = |allowed: fn(&crate::PageState) -> bool| state.is_some_and(allowed);
    let (loading, back, forward) = (
        can(|state| state.loading),
        can(|state| state.can_go_back),
        can(|state| state.can_go_forward),
    );
    let load = if loading {
        history_button(
            "page.reload",
            "Stop loading",
            Icon::Stop,
            Action::PageStop,
            true,
        )
    } else {
        history_button(
            "page.reload",
            "Reload",
            Icon::Reload,
            Action::PageReload,
            true,
        )
    };
    vec![
        history_button(
            "page.back",
            "Back",
            Icon::ChevronLeft,
            Action::PageBack,
            back,
        ),
        history_button(
            "page.forward",
            "Forward",
            Icon::ChevronRight,
            Action::PageForward,
            forward,
        ),
        load,
    ]
}

/// The address the page shows, or nothing for a page that has none yet.
fn address(app: &App, page: &Entity) -> Control {
    let live = app.page_state(&page.id).and_then(|state| state.url.clone());
    let stored = match &page.kind {
        Kind::Page(page) => page.url.clone(),
        Kind::Text(_) | Kind::Shape(_) | Kind::Drawing(_) | Kind::Group(_) | Kind::File(_) => {
            String::new()
        }
    };
    let url = live.unwrap_or(stored);
    Control::Field(Field {
        id: ControlId::new("page.url"),
        label: "Page address".into(),
        caption: None,
        value: if url == "about:blank" {
            String::new()
        } else {
            url
        },
        placeholder: Some("Type a URL".into()),
        width: FieldWidth::Wide,
        submit: FieldSubmit::PageUrl,
    })
}

/// A page's size as a field shows it.
fn pixels(size: f64) -> String {
    format!("{size:.0}")
}

/// The width and height of the one page selected, to type a custom size in.
fn size_fields(id: &ControlId, page: &Entity) -> DropdownSection {
    let field = |name: &'static str, caption: &'static str, value, submit| {
        Control::Field(Field {
            id: id.child(name),
            label: format!("Page {name}").into(),
            caption: Some(caption.into()),
            value: pixels(value),
            placeholder: None,
            width: FieldWidth::Short,
            submit,
        })
    };
    DropdownSection::Controls(vec![
        field("width", "W", page.rect.width, FieldSubmit::ViewportWidth),
        field("height", "H", page.rect.height, FieldSubmit::ViewportHeight),
    ])
}

fn size_dropdown(app: &App, single: Option<&Entity>) -> Control {
    let id = ControlId::new("page.size");
    let preset = read::viewport_preset(app);
    let custom = single.map(|_| {
        (
            preset.is_none(),
            Action::SetProperty(Property::CustomViewport),
        )
    });
    let mut sections = preset_sections(
        &id,
        preset,
        None,
        |index| Action::SetProperty(Property::ViewportPreset(index)),
        custom,
    );
    // Several pages have no one custom size to keep.
    if let Some(page) = single {
        sections.push(size_fields(&id, page));
    }
    let named = preset.and_then(|index| VIEWPORT_PRESETS.get(usize::try_from(index).ok()?));
    let summary = match (named, single.is_some()) {
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

/// `/Users/<name>` shortened to `~`.
fn tilde(path: &str) -> String {
    match path.strip_prefix("/Users/") {
        Some(rest) => match rest.split_once('/') {
            Some((_, tail)) => format!("~/{tail}"),
            None => "~".to_owned(),
        },
        None => path.to_owned(),
    }
}

/// The most characters of a folder the repo row shows, so the path stays
/// clear of the row's trailing text.
const FOLDER_CHARS: usize = 28;

/// A folder for the repo row: `~` for the home folder, and the end of a long
/// path after an ellipsis, since the end is what tells two repos apart.
fn short_folder(path: &str) -> String {
    let path = tilde(path);
    let length = path.chars().count();
    if length <= FOLDER_CHARS {
        return path;
    }
    let tail: String = path.chars().skip(length - (FOLDER_CHARS - 1)).collect();
    format!("\u{2026}{tail}")
}

/// Which repo the page's origin writes to. A page with no origin (`data:`,
/// `file:`) has nothing to bind and no control.
fn repo_control(app: &App, page: &Entity) -> Option<Control> {
    let live = app
        .page_state(&page.id)
        .and_then(|state| state.url.as_deref());
    let stored = match &page.kind {
        Kind::Page(page) => Some(page.url.as_str()),
        Kind::Text(_) | Kind::Shape(_) | Kind::Drawing(_) | Kind::Group(_) | Kind::File(_) => None,
    };
    let origin = live
        .and_then(origin_of)
        .or_else(|| stored.and_then(origin_of))?;
    let bound = app
        .repos()
        .binding(&origin)
        .map(|binding| binding.repo_path);
    let id = ControlId::new("page.repo");
    let pick = Action::Repo(RepoAction::Pick(Some(origin.clone())));
    let row = |name: &str, label: String, trailing: Option<&str>, enabled, action| DropdownOption {
        id: id.child(name),
        face: Face::text(label.clone()),
        label: label.into(),
        trailing: trailing.map(|text| text.to_owned().into()),
        chord: None,
        selected: false,
        enabled,
        action,
    };
    let mut options = vec![
        row("origin", origin.clone(), None, false, pick.clone()),
        match bound {
            Some(path) => row(
                "folder",
                short_folder(path),
                Some("Change\u{2026}"),
                true,
                pick,
            ),
            None => row(
                "folder",
                "No repo linked".to_owned(),
                Some("Choose\u{2026}"),
                true,
                pick,
            ),
        },
    ];
    if bound.is_some() {
        let unlink = Action::Repo(RepoAction::Unlink(origin));
        options.push(row("unlink", "Unlink".to_owned(), None, true, unlink));
    }
    Some(Control::Dropdown(Dropdown {
        label: match bound {
            Some(path) => format!("Repo: {}", tilde(path)).into(),
            None => "Link a repo".into(),
        },
        summary: Face::icon(Icon::Repo),
        content: vec![DropdownSection::Options {
            layout: OptionLayout::List,
            options,
        }],
        id,
    }))
}

pub(super) fn popup(app: &App, entities: &[&Entity]) -> PopupModel {
    let single = match entities {
        [one] => Some(*one),
        _ => None,
    };
    let controls = if let Some(page) = single {
        groups(vec![
            history(app, page),
            vec![address(app, page)],
            vec![size_dropdown(app, single)],
            vec![frame_toggle(app, "Device frame"), rotate(app)],
            vec![scheme(app)],
            repo_control(app, page).into_iter().collect(),
            Actions {
                sync: app.selection_synced(),
                annotate: false,
                ..Actions::all("page", 1)
            }
            .controls(),
        ])
    } else {
        groups(vec![
            vec![size_dropdown(app, None)],
            vec![frame_toggle(app, "Toggle device frame for selected pages")],
            Actions {
                sync: app.selection_synced(),
                focus: false,
                ..Actions::all(&format!("{} pages", entities.len()), entities.len())
            }
            .controls(),
        ])
    };
    let align = if single.is_some() {
        Align::Stretch
    } else {
        Align::Center
    };
    PopupModel {
        anchor: super::over_titled(entities, align),
        controls,
    }
}
