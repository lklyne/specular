//! The dock's controls for pages (`PagePopup.tsx`): history, reload and the address
//! (several pages keep the address alone), then the size list, which also holds the device frame, orientation and
//! color scheme, and the chain button that syncs the selected pages, and the
//! repo control of a single page. Electron also infers a repo from a running dev server's base URL;
//! this app runs no dev servers, so the origin's binding is the only source.

use specular_agent::origin_of;
use specular_doc::{
    ColorScheme, DeviceFrameColor, DeviceFrameStyle, Entity, Kind, PRESET_MAIN, PRESET_MENU,
    VIEWPORT_PRESETS,
};

use super::super::build::groups;
use super::super::{
    Button, Control, ControlId, ControlsModel, Dropdown, DropdownOption, DropdownSection, Face,
    Field, FieldSubmit, FieldWidth, Icon, OptionLayout, PAGE_URL,
};
use super::actions::Actions;
use crate::property::read;
use crate::{Action, App, Orientation, Property, RepoAction, binding_of};

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
pub(super) fn preset_sections(
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

/// The address `page` shows, or nothing for a page that has none yet.
fn shown(app: &App, page: &Entity) -> String {
    let live = app.page_state(&page.id).and_then(|state| state.url.clone());
    let stored = match &page.kind {
        Kind::Page(page) => page.url.clone(),
        Kind::Text(_) | Kind::Shape(_) | Kind::Drawing(_) | Kind::Group(_) | Kind::File(_) => {
            String::new()
        }
    };
    let url = live.unwrap_or(stored);
    if url == "about:blank" {
        String::new()
    } else {
        url
    }
}

/// The address field of `pages`. It holds the address they all show, and is
/// empty where they differ. What is typed takes every one of them there.
fn address(app: &App, pages: &[&Entity]) -> Control {
    let mut shown = pages.iter().map(|page| shown(app, page));
    let first = shown.next().unwrap_or_default();
    let shared = shown.all(|url| url == first);
    Control::Field(Field {
        id: ControlId::new(PAGE_URL),
        label: "Page address".into(),
        caption: None,
        value: if shared { first } else { String::new() },
        placeholder: Some(
            if shared {
                "Search or type a URL"
            } else {
                "Type a URL for every page"
            }
            .into(),
        ),
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
    // the frame, and a device has a colour only while it is drawn.
    sections.push(frame_choice(app));
    sections.extend(read::any_device_frame(app).then(|| frame_color_choice(app)));
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

/// The device frame the pages are drawn in.
fn frame_choice(app: &App) -> DropdownSection {
    choice(
        "page.frame",
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

/// The colour the devices are drawn in.
fn frame_color_choice(app: &App) -> DropdownSection {
    choice(
        "page.frame-color",
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
            vec![address(app, entities)],
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
            vec![address(app, entities)],
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
