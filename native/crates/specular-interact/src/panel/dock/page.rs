//! The dock's controls for pages (`PagePopup.tsx`): history, reload and the address,
//! then the size list, which also holds the device frame, orientation and
//! color scheme, and the chain button that syncs the selected pages, and the
//! repo control of a single page. Electron also infers a repo from a running dev server's base URL;
//! this app runs no dev servers, so the origin's binding is the only source.

use specular_agent::origin_of;
use specular_doc::{ColorScheme, DeviceFrameStyle, Entity, Kind, VIEWPORT_PRESETS};

use super::super::build::{button, groups, toggle};
use super::super::{
    Button, Control, ControlId, ControlsModel, Dropdown, DropdownOption, DropdownSection, Face,
    Field, FieldSubmit, FieldWidth, Icon, OptionLayout, PAGE_URL,
};
use super::actions::Actions;
use crate::property::read;
use crate::{Action, App, PageStep, Property, RepoAction, binding_of};

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
/// its flag is, runs its action, and has its fields at its far end. `verb`
/// starts each row's accessible name ("Add iPhone SE").
pub(super) fn preset_sections(
    id: &ControlId,
    selected: Option<u32>,
    verb: Option<&str>,
    pick: impl Fn(u32) -> Action,
    custom: Option<(bool, Action, Vec<Field>)>,
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
                        fields: Vec::new(),
                        chord: None,
                        selected: selected == Some(index),
                        enabled: true,
                        action: pick(index),
                    })
                })
                .collect(),
        })
        .collect();
    if let Some((selected, action, fields)) = custom {
        sections.push(DropdownSection::Options {
            layout: OptionLayout::List,
            options: vec![DropdownOption {
                id: id.child("custom"),
                label: named(if verb.is_some() { "custom" } else { "Custom" }),
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
        id: ControlId::new(PAGE_URL),
        label: "Page address".into(),
        caption: None,
        value: if url == "about:blank" {
            String::new()
        } else {
            url
        },
        placeholder: Some("Search or type a URL".into()),
        width: FieldWidth::Wide,
        submit: FieldSubmit::PageUrl,
    })
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

fn size_dropdown(app: &App, single: Option<&Entity>) -> Control {
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
    // the frame.
    sections.push(DropdownSection::Controls(match single {
        Some(_) => vec![frame_toggle(app, "Device frame"), rotate(), scheme(app)],
        None => vec![frame_toggle(app, "Toggle device frame for selected pages")],
    }));
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

/// A glyph and the word for it, as a control in the size list shows them.
fn labelled(icon: Icon, text: &'static str) -> Face {
    Face {
        icon: Some(icon),
        ..Face::text(text)
    }
}

/// The frame toggle steps through none, clay and realistic, and is on
/// while every page has a frame. It names the frame the pages share.
fn frame_toggle(app: &App, label: &'static str) -> Control {
    let shown = read::device_frame(app);
    let name = match shown {
        Some(None) => "No frame",
        Some(Some(DeviceFrameStyle::Clay)) => "Clay",
        Some(Some(DeviceFrameStyle::Realistic)) => "Realistic",
        None => "Frame",
    };
    Control::Toggle(toggle(
        ControlId::new("page.frame"),
        label,
        labelled(Icon::Device, name),
        matches!(shown, Some(Some(_))),
        Action::StepPage(PageStep::Frame),
    ))
}

fn rotate() -> Control {
    button(
        ControlId::new("page.rotate"),
        "Rotate viewport",
        labelled(Icon::Rotate, "Rotate"),
        Action::StepPage(PageStep::Rotate),
    )
}

/// The color scheme button steps through system, light and dark.
fn scheme(app: &App) -> Control {
    let (name, icon) = match read::color_scheme(app) {
        Some(Some(ColorScheme::Light)) => ("Light", Icon::SchemeLight),
        Some(Some(ColorScheme::Dark)) => ("Dark", Icon::SchemeDark),
        Some(None) | None => ("System", Icon::SchemeSystem),
    };
    button(
        ControlId::new("page.scheme"),
        format!("Color scheme: {name}. Click to change."),
        labelled(icon, name),
        Action::StepPage(PageStep::Scheme),
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
        fields: Vec::new(),
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

pub(super) fn controls(app: &App, entities: &[&Entity]) -> ControlsModel {
    let single = match entities {
        [one] => Some(*one),
        _ => None,
    };
    let controls = if let Some(page) = single {
        groups(vec![
            history(app, page),
            vec![address(app, page)],
            vec![size_dropdown(app, single)],
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
            Actions {
                sync: app.selection_synced(),
                ..Actions::all(&format!("{} pages", entities.len()), entities.len())
            }
            .controls(),
        ])
    };
    ControlsModel { controls }
}
