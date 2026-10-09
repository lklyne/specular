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
    Control, ControlId, Dropdown, DropdownOption, DropdownSection, Face, Icon, OptionLayout,
    SidebarButton, ThemeButton, Toggle, ToolButton, ToolbarModel, ToolbarSection,
};
use crate::menu::{tool_action, tool_label};
use crate::{Action, App, ChatAction, Lens, Theme, Tool, binding_of};

/// The zoom levels the readout offers, in percent.
const ZOOM_LEVELS: [u16; 7] = [10, 25, 50, 75, 100, 150, 200];

const fn tool_name(tool: Tool) -> &'static str {
    match tool {
        Tool::Select => "select",
        Tool::AddPage => "page",
        Tool::AddText => "text",
        Tool::AddSticky => "sticky",
        Tool::AddDocument => "document",
        Tool::AddShape => "shape",
        Tool::Draw => "draw",
        Tool::Comment => "comment",
        Tool::Inspect => "inspect",
    }
}

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
    }
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
        Tool::Draw | Tool::Comment | Tool::Inspect => true,
        Tool::Select
        | Tool::AddPage
        | Tool::AddText
        | Tool::AddSticky
        | Tool::AddDocument
        | Tool::AddShape => false,
    }
}

fn button(app: &App, tool: Tool) -> ToolButton {
    let active = app.session.tool == tool;
    let (icon, tint) = glyph(app, tool);
    ToolButton {
        id: ControlId::new("tool").child(tool_name(tool)),
        tool,
        label: tool_label(tool).into(),
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

/// The button that moves the theme on: it shows the choice in force
/// (`THEME_MODE_ICON`, `THEME_MODE_LABEL`).
fn theme_button(app: &App) -> ThemeButton {
    let theme = app.theme();
    ThemeButton {
        id: ControlId::new("theme"),
        label: format!("{} theme", theme.label()).into(),
        icon: match theme {
            Theme::System => Icon::SchemeSystem,
            Theme::Light => Icon::SchemeLight,
            Theme::Dark => Icon::SchemeDark,
        },
        action: Action::SetTheme(theme.next()),
    }
}

/// A lens as the control names it: its id, the word on its segment and
/// its tooltip.
const fn lens_words(lens: Lens) -> (&'static str, &'static str, &'static str) {
    match lens {
        Lens::Fill => ("fill", "Fill", "Fill the window"),
        Lens::Device => ("device", "Device", "Device size, fitted to the window"),
        Lens::Canvas => ("canvas", "Canvas", "On the canvas, free to pan and zoom"),
    }
}

/// The lens of the tab showing and the eye. Nothing on the Canvas tab.
fn view_controls(app: &App) -> Vec<Control> {
    let Some(now) = app.lens() else {
        return Vec::new();
    };
    let id = ControlId::new("view.lens");
    let lenses = Lens::ALL.into_iter().map(|lens| {
        let (name, word, hint) = lens_words(lens);
        Control::Toggle(Toggle {
            id: id.child(name),
            label: hint.into(),
            face: Face::text(word),
            on: lens == now,
            enabled: true,
            chord: None,
            action: Action::SetLens(lens),
        })
    });
    let open = app.shows_others();
    let eye = Toggle {
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
    };
    lenses
        .chain([Control::Separator, Control::Toggle(eye)])
        .collect()
}

/// The toolbar for `app` as it is now.
pub fn toolbar(app: &App) -> ToolbarModel {
    let mut sections: Vec<ToolbarSection> = GROUPS
        .iter()
        .map(|tools| ToolbarSection::Tools(tools.iter().map(|&tool| button(app, tool)).collect()))
        .collect();
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
    ToolbarModel {
        theme: theme_button(app),
        chat,
        sections,
        view: view_controls(app),
    }
}
