//! What the window draws: hiding and showing its regions and controls over
//! the layout file, and going back to the shipped layout.

use serde_json::Value;

use super::args::{act, always, choice, maybe_text, text};
use super::{CommandSpec, Run, RunError};
use crate::panel::{ContextMenu, MenuTarget};
use crate::{Action, CanvasId, LayoutAction, OverlayAction, OverlaySpec};

/// The `id` argument: a node's, an overlay's or a control's id.
fn id(args: &Value) -> Result<String, RunError> {
    let id = text(args, "id")?;
    if id.is_empty() {
        return Err(RunError::Args(
            "`id` names a block, an overlay or a control".to_owned(),
        ));
    }
    Ok(id)
}

fn layout(action: LayoutAction) -> Result<Run, RunError> {
    act(Action::Layout(action))
}

/// The window's regions and controls.
pub(super) const UI: &[CommandSpec] = &[
    CommandSpec {
        name: "ui.toggle",
        label: "",
        args: "{id: <layout node id>|dock|menu|share|toast|palette|welcome|<control id>}",
        about: "Hide a block of the layout, an overlay or a control if it is shown, show it if hidden.",
        enabled: always,
        build: |args| layout(LayoutAction::Toggle(id(args)?)),
    },
    CommandSpec {
        name: "ui.show",
        label: "",
        args: "{id: <layout node id>|dock|menu|share|toast|palette|welcome|<control id>}",
        about: "Show a block of the layout, an overlay or a control.",
        enabled: always,
        build: |args| layout(LayoutAction::Show(id(args)?)),
    },
    CommandSpec {
        name: "ui.hide",
        label: "",
        args: "{id: <layout node id>|dock|menu|share|toast|palette|welcome|<control id>}",
        about: "Hide a block of the layout, an overlay or a control, giving its room to its neighbours.",
        enabled: always,
        build: |args| layout(LayoutAction::Hide(id(args)?)),
    },
    CommandSpec {
        name: "ui.reset",
        label: "",
        args: "",
        about: "Set aside what was hidden and shown, and go back to the shipped default layout.",
        enabled: always,
        build: |_| layout(LayoutAction::Reset),
    },
    CommandSpec {
        name: "layout.switch",
        label: "Next Layout",
        args: "{name?: layout name}",
        about: "Show the layout named, `layouts/<name>.json`, or with no name the next one.",
        enabled: always,
        build: |args| layout(LayoutAction::Switch(maybe_text(args, "name"))),
    },
];

/// The notices a toast shows.
pub(super) const NOTICE: &[CommandSpec] = &[CommandSpec {
    name: "notice.dismiss",
    label: "",
    args: "{id: notice id}",
    about: "Take away a page's notice in the `notices` key; it comes back if the page fails again.",
    enabled: always,
    build: |args| act(Action::DismissNotice(text(args, "id")?)),
}];

/// Mod overlays: dialogs, popups, drawers and windows over the window.
pub(super) const OVERLAY: &[CommandSpec] = &[
    CommandSpec {
        name: "overlay.open",
        label: "",
        args: "{mod: mod name, id?: overlay id, props?: {}, kind?: dialog|popup|drawer|window, width?: points, height?: points, at?: [x, y], edge?: left|right|top|bottom, frame?: none}",
        about: "Open a copy of a mod over the window; one open by the same id moves to the top with the new spec.",
        enabled: always,
        build: |args| {
            let spec = OverlaySpec::parse(args).map_err(RunError::Args)?;
            act(Action::Overlay(OverlayAction::Open(Box::new(spec))))
        },
    },
    CommandSpec {
        name: "overlay.close",
        label: "",
        args: "{id?: overlay id}",
        about: "Close the overlay named, or the top one.",
        enabled: always,
        build: |args| {
            act(Action::Overlay(OverlayAction::Close(maybe_text(
                args, "id",
            ))))
        },
    },
];

/// The `at` argument: a point in window points.
fn point(args: &Value) -> Result<glam::Vec2, RunError> {
    let at = (args.get("at").and_then(Value::as_array))
        .filter(|at| at.len() == 2)
        .and_then(|at| {
            Some(glam::Vec2::new(
                at[0].as_f64()? as f32,
                at[1].as_f64()? as f32,
            ))
        });
    at.ok_or_else(|| RunError::Args("`at` is [x, y] in window points".to_owned()))
}

/// The context menu of something a mod draws.
pub(super) const MENU: &[CommandSpec] = &[CommandSpec {
    name: "menu.open",
    label: "",
    args: "{target: canvas, id: canvas id, at: [x, y]}",
    about: "Open the context menu of a canvas of the sidebar with its corner at a point in the window, as a right press on its row does.",
    enabled: always,
    build: |args| {
        choice(args, "target", &[("canvas", ())])?;
        let target = MenuTarget::Canvas(CanvasId::new(text(args, "id")?));
        act(Action::OpenMenu(ContextMenu {
            target,
            at: point(args)?,
        }))
    },
}];
