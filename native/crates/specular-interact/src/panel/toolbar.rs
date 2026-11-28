//! [`toolbar`]: the tool buttons and the zoom readout (`toolbarSections.tsx`).
//!
//! The Electron toolbar also has a hand tool, which has no [`Action`] here.
//! The right panel's toggle is on the
//! model when the shell has a right panel.
//!
//! A button arms its tool and leaves the tool's defaults alone: the key of
//! the draw tool also picks the pen, and a click must not.

use specular_doc::Color;

use super::{
    Button, Control, ControlId, Dropdown, DropdownOption, DropdownSection, Face, Icon, Label,
    OptionLayout, SidebarButton, ThemeButton, Toggle, ToolButton, ToolbarModel, ToolbarSection,
};
use crate::menu::{tool_action, tool_label};
use crate::mods::ModToolId;
use crate::showing;
use crate::{Action, App, Appearance, ChatAction, Lens, Showing, Tool, binding_of};

/// The zoom levels the readout offers, in percent.
const ZOOM_LEVELS: [u16; 7] = [10, 25, 50, 75, 100, 150, 200];

/// The glyph of `tool`, and the color it is drawn in when that shows the
/// tool's current default.
fn glyph(app: &App, tool: Tool) -> (Icon, Option<Color>) {
    let defaults = app.tool_defaults();
    match tool {
        Tool::Select => (Icon::SelectTool, None),
        Tool::AddPage => (Icon::PageTool, None),
        Tool::AddText => (Icon::TextTool, None),
        Tool::AddSticky => (Icon::StickyTool, Some(defaults.sticky.color.clone())),
        Tool::AddDocument => (Icon::DocumentTool, None),
        Tool::AddShape => (Icon::ShapeTool, Some(defaults.shape.color.clone())),
        Tool::Draw => {
            let icon = match defaults.draw.brush {
                specular_doc::BrushType::Pen => Icon::DrawPenTool,
                specular_doc::BrushType::Highlight => Icon::DrawHighlightTool,
            };
            (icon, Some(defaults.draw.color.clone()))
        }
        Tool::Comment => (Icon::CommentTool, None),
        Tool::Inspect => (Icon::InspectTool, None),
        Tool::Mod(id) => (mod_icon(app, id), None),
    }
}

/// A mod tool's glyph: the app glyph its `mod.json` names, else a pen.
fn mod_icon(app: &App, id: ModToolId) -> Icon {
    let spec = app.mods.catalog.tool(id).map(|(_, spec)| spec);
    Icon::named_or(spec.and_then(|spec| spec.icon.as_deref()), Icon::PenLine)
}

/// The word a tool's control id ends in, and what the toolbar calls it. A
/// mod tool's are its `<mod>/<tool>` word and its `mod.json` title.
fn naming(app: &App, tool: Tool) -> (Label, Label) {
    if let Tool::Mod(id) = tool
        && let Some((installed, spec)) = app.mods.catalog.tool(id)
    {
        let word = format!("{}/{}", installed.name, spec.name);
        return (word.into(), spec.title.clone().into());
    }
    (tool.name().into(), tool_label(tool).into())
}

fn zoom(app: &App) -> Dropdown {
    let percent = (app.session.camera.zoom * 100.0).round();
    let id = ControlId::new("zoom");
    let options = (ZOOM_LEVELS.into_iter())
        .map(|level| {
            // 100% is the View menu's own item, so the two share a key.
            let action = if level == 100 {
                Action::ZoomReset
            } else {
                Action::ZoomTo(level)
            };
            DropdownOption {
                id: id.child(level),
                label: format!("Zoom to {level}%").into(),
                face: Face::text(format!("{level}%")),
                trailing: None,
                fields: Vec::new(),
                chord: binding_of(&action).map(|binding| binding.chord),
                selected: (percent - f32::from(level)).abs() < 0.5,
                enabled: true,
                action,
            }
        })
        .collect();
    Dropdown {
        label: "Zoom".into(),
        summary: Face::text(format!("{percent}%")),
        content: vec![DropdownSection::Options {
            layout: OptionLayout::List,
            options,
        }],
        id,
    }
}

/// The tool buttons in toolbar order, a divider between groups: selecting,
/// creating, annotating.
const GROUPS: [&[Tool]; 3] = [
    &[Tool::Select],
    &[
        Tool::Draw,
        Tool::AddSticky,
        Tool::AddShape,
        Tool::AddPage,
        Tool::AddText,
        Tool::AddDocument,
    ],
    &[Tool::Comment, Tool::Inspect],
];

/// Whether pressing the button of `tool` while it is armed puts it down.
const fn toggles(tool: Tool) -> bool {
    match tool {
        Tool::Draw | Tool::Comment | Tool::Inspect | Tool::Mod(_) => true,
        Tool::Select
        | Tool::AddPage
        | Tool::AddText
        | Tool::AddSticky
        | Tool::AddDocument
        | Tool::AddShape => false,
    }
}

fn button(app: &App, tool: Tool) -> ToolButton {
    // A mod's shape is the shape tool in hand, under another form.
    let shape = matches!(app.session.tool, Tool::Mod(id) if app.mods.catalog.is_shape(id));
    let active = app.session.tool == tool || (shape && tool == Tool::AddShape);
    let (icon, tint) = glyph(app, tool);
    let (word, label) = naming(app, tool);
    ToolButton {
        id: ControlId::new("tool").child(word),
        tool,
        label,
        icon,
        tint,
        active,
        chord: binding_of(&tool_action(tool)).map(|binding| binding.chord),
        action: Action::SetTool(if active && toggles(tool) {
            Tool::Select
        } else {
            tool
        }),
    }
}

/// The button that flips the theme: it shows what is drawn now and says
/// what a press draws instead.
fn theme_button(app: &App) -> ThemeButton {
    let (label, icon) = match app.appearance() {
        Appearance::Light => ("Switch to dark theme", Icon::SchemeLight),
        Appearance::Dark => ("Switch to light theme", Icon::SchemeDark),
    };
    ThemeButton {
        id: ControlId::new("theme"),
        label: label.into(),
        icon,
        action: Action::SetTheme(app.theme.toggled()),
    }
}

/// A lens as the control shows it: the word on its segment and its tooltip.
const fn lens_words(lens: Lens) -> (&'static str, &'static str) {
    match lens {
        Lens::Fill => ("Fill", "Fill the window"),
        Lens::Device => ("Device", "Device size, fitted to the window"),
    }
}

/// The button that opens the one page or Document selected in a tab of
/// its own.
fn open_tab(app: &App) -> Option<Control> {
    let item = showing::openable(app)?;
    Some(Control::Button(Button {
        id: ControlId::new("view.open"),
        label: "Open in a tab".into(),
        face: Face::icon(Icon::Expand),
        enabled: true,
        chord: None,
        action: Action::OpenTab(item.clone()),
    }))
}

/// Minimize and the lens of the tab showing. On the Canvas tab, the button
/// that opens the item selected in a tab, or nothing.
fn view_controls(app: &App) -> Vec<Control> {
    let Some(now) = app.lens() else {
        return open_tab(app).into_iter().collect();
    };
    let minimize = Control::Button(Button {
        id: ControlId::new("view.minimize"),
        label: "Minimize".into(),
        face: Face::icon(Icon::Minimize),
        enabled: true,
        chord: None,
        action: Action::Show(Showing::Canvas),
    });
    let id = ControlId::new("view.lens");
    let lenses = Lens::ALL.into_iter().map(|lens| {
        let (word, hint) = lens_words(lens);
        Control::Toggle(Toggle {
            id: id.child(lens.key()),
            label: hint.into(),
            face: Face::text(word),
            on: lens == now,
            enabled: true,
            chord: None,
            action: Action::SetLens(lens),
        })
    });
    // The eye is Device's: Fill has no canvas around its item to show.
    let eye = (now == Lens::Device).then(|| {
        let open = app.shows_others();
        Control::Toggle(Toggle {
            id: ControlId::new("view.others"),
            label: if open {
                "Hide other items"
            } else {
                "Show other items"
            }
            .into(),
            face: Face::icon(if open { Icon::Eye } else { Icon::EyeOff }),
            on: open,
            enabled: true,
            chord: None,
            action: Action::ShowOthers(!open),
        })
    });
    eye.into_iter().chain(lenses).chain([minimize]).collect()
}

/// The toolbar for `app` as it is now.
pub fn toolbar(app: &App) -> ToolbarModel {
    let mut sections: Vec<ToolbarSection> = GROUPS
        .iter()
        .map(|tools| ToolbarSection::Tools(tools.iter().map(|&tool| button(app, tool)).collect()))
        .collect();
    // The installed mods' tools, a group of their own after the app's.
    // A shape is listed under the shape tool, not here.
    let mods: Vec<ToolButton> = (app.mods.catalog.tools())
        .filter(|(_, _, spec)| !spec.is_shape())
        .map(|(id, ..)| button(app, Tool::Mod(id)))
        .collect();
    if !mods.is_empty() {
        sections.push(ToolbarSection::Tools(mods));
    }
    sections.push(ToolbarSection::Zoom(zoom(app)));
    let chat = app.chat_view().available().then(|| {
        let open = app.chat_view().shown();
        SidebarButton {
            id: ControlId::new("chat.toggle"),
            label: if open {
                "Collapse right panel"
            } else {
                "Expand right panel"
            }
            .into(),
            icon: Icon::PanelRight,
            open,
            action: Action::Chat(ChatAction::Toggle),
        }
    });
    let lights = !app.sidebar_holds_title();
    let sidebar = (app.regions().sidebar && lights).then(|| crate::sidebar::sidebar_toggle(app));
    ToolbarModel {
        theme: theme_button(app),
        chat,
        sidebar,
        lights,
        share: super::share::button(app),
        sections,
        tools: app.tools_place(),
        view: view_controls(app),
    }
}
