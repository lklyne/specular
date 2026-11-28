//! The tools, and the View menu's commands: zoom, the camera, the theme and
//! the sidebar.

use glam::Vec2;
use serde_json::Value;
use specular_core::Camera;
use specular_doc::{BrushType, ItemId, ShapeKind};

use super::args::{act, always, can, choice, entity, idle, number, text};
use super::{CommandSpec, Run, RunError};
use crate::{Action, SidebarAction, Theme, Tool, ToolDefaultPatch};

fn tool(args: &Value) -> Result<Run, RunError> {
    let name = text(args, "tool")?;
    // A mod's tool, `<mod>/<tool>`, is looked up when it runs: what is
    // installed is the app's to know.
    if name.contains('/') {
        return act(Action::Mod(Box::new(crate::mods::ModAction::SetTool(name))));
    }
    let tool = Tool::named(&name).ok_or_else(|| {
        let names: Vec<&str> = Tool::ALL.iter().map(|tool| tool.name()).collect();
        RunError::Args(format!("`tool` is one of {}", names.join(", ")))
    })?;
    act(Action::SetTool(tool))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "a camera is f32, and a script's numbers are small"
)]
fn camera(args: &Value) -> Result<Run, RunError> {
    let pan = Vec2::new(number(args, "x")? as f32, number(args, "y")? as f32);
    act(Action::SetCamera(Camera::new(
        pan,
        number(args, "zoom")? as f32,
    )))
}

fn zoom(args: &Value) -> Result<Run, RunError> {
    let percent = number(args, "percent")?;
    let percent = u16::try_from(percent.round() as i64)
        .ok()
        .filter(|percent| *percent > 0)
        .ok_or_else(|| RunError::Args("`percent` is a whole number above 0".to_owned()))?;
    act(Action::ZoomTo(percent))
}

/// Arming a tool, and the tool variants the palette lists.
pub(super) const TOOL: &[CommandSpec] = &[
    CommandSpec {
        name: "tool.set",
        label: "",
        args: "{tool: select|page|text|sticky|document|shape|draw|comment|inspect|<mod>/<tool>}",
        about: "Arm a tool.",
        enabled: always,
        build: tool,
    },
    CommandSpec {
        name: "tool.ellipse",
        label: "Ellipse",
        args: "",
        about: "Arm the shape tool with ellipses.",
        enabled: idle,
        build: |_| {
            act(Action::SetToolVariant(ToolDefaultPatch::ShapeKind(
                ShapeKind::Ellipse,
            )))
        },
    },
    CommandSpec {
        name: "tool.diamond",
        label: "Diamond",
        args: "",
        about: "Arm the shape tool with diamonds.",
        enabled: idle,
        build: |_| {
            act(Action::SetToolVariant(ToolDefaultPatch::ShapeKind(
                ShapeKind::Diamond,
            )))
        },
    },
    CommandSpec {
        name: "tool.highlighter",
        label: "Highlighter",
        args: "",
        about: "Arm the draw tool with the highlighter.",
        enabled: idle,
        build: |_| {
            act(Action::SetToolVariant(ToolDefaultPatch::Brush(
                BrushType::Highlight,
            )))
        },
    },
];

/// Zoom, the camera, the theme and the sidebar.
pub(super) const VIEW: &[CommandSpec] = &[
    CommandSpec {
        name: "view.zoom-in",
        label: "Zoom in",
        args: "",
        about: "Zoom in one step about the middle of the viewport.",
        enabled: idle,
        build: |_| act(Action::ZoomIn),
    },
    CommandSpec {
        name: "view.zoom-out",
        label: "Zoom out",
        args: "",
        about: "Zoom out one step about the middle of the viewport.",
        enabled: idle,
        build: |_| act(Action::ZoomOut),
    },
    CommandSpec {
        name: "view.zoom-reset",
        label: "Zoom to 100%",
        args: "",
        about: "Zoom to 100% about the middle of the viewport.",
        enabled: idle,
        build: |_| act(Action::ZoomReset),
    },
    CommandSpec {
        name: "view.zoom-to-fit",
        label: "Zoom to fit",
        args: "",
        about: "Show everything on the canvas, centred.",
        enabled: |app| can(app, &Action::ZoomToFit),
        build: |_| act(Action::ZoomToFit),
    },
    CommandSpec {
        name: "view.zoom",
        label: "",
        args: "{percent: number}",
        about: "Zoom to this many percent about the middle of the viewport.",
        enabled: idle,
        build: zoom,
    },
    CommandSpec {
        name: "view.camera",
        label: "",
        args: "{x: number, y: number, zoom: number}",
        about: "Move the camera: the pan in screen pixels, and the zoom.",
        enabled: idle,
        build: camera,
    },
    CommandSpec {
        name: "view.reveal",
        label: "",
        args: "{id: entity id}",
        about: "Select an entity and pan it into view.",
        enabled: idle,
        build: |args| {
            let item = ItemId::Entity(entity(args, "id")?);
            act(Action::Reveal {
                select: vec![item.clone()],
                focus: item,
            })
        },
    },
    CommandSpec {
        name: "view.theme",
        label: "",
        args: "{theme: light|dark|system}",
        about: "Choose the theme and save the choice.",
        enabled: always,
        build: |args| {
            let theme = choice(
                args,
                "theme",
                &[Theme::Light, Theme::Dark, Theme::System].map(|theme| (theme.key(), theme)),
            )?;
            act(Action::SetTheme(theme))
        },
    },
    CommandSpec {
        name: "view.toggle-sidebar",
        label: "Toggle sidebar",
        args: "",
        about: "Show the sidebar, or hide it.",
        enabled: always,
        build: |_| act(Action::Sidebar(SidebarAction::Toggle)),
    },
];
