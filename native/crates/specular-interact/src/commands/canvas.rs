//! The window's own commands, the space's canvases, sharing, and pressing a
//! control by name.

use serde_json::Value;

use super::args::{act, always, choice, idle, maybe_text, switch, text};
use super::{CommandSpec, Run, RunError};
use crate::{
    Action, ApiRun, CanvasAction, CanvasId, ControlId, LinkScope, ShareAction, ShellCommand,
};

#[expect(
    clippy::unnecessary_wraps,
    reason = "a command's build returns it as its own answer"
)]
const fn shell(command: ShellCommand) -> Result<Run, RunError> {
    Ok(Run::Shell(command))
}

fn canvas(args: &Value) -> Option<CanvasId> {
    maybe_text(args, "canvas").map(CanvasId::new)
}

/// The window's own commands, in the order of its keys.
pub(super) const APP: &[CommandSpec] = &[
    CommandSpec {
        name: "app.quit",
        label: "Quit",
        args: "",
        about: "Quit Specular.",
        enabled: always,
        build: |_| shell(ShellCommand::Quit),
    },
    CommandSpec {
        name: "app.open-space",
        label: "Open space",
        args: "",
        about: "Choose another space folder to open.",
        enabled: always,
        build: |_| shell(ShellCommand::OpenSpace),
    },
    CommandSpec {
        name: "app.open",
        label: "Open",
        args: "",
        about: "Choose a canvas file to open.",
        enabled: always,
        build: |_| shell(ShellCommand::OpenCanvas),
    },
    CommandSpec {
        name: "app.save",
        label: "Save",
        args: "",
        about: "Write every canvas now.",
        enabled: always,
        build: |_| shell(ShellCommand::Save),
    },
    CommandSpec {
        name: "app.settings",
        label: "Settings",
        args: "",
        about: "Open the settings.",
        enabled: always,
        build: |_| shell(ShellCommand::Settings),
    },
    CommandSpec {
        name: "app.hide",
        label: "Hide",
        args: "",
        about: "Hide Specular.",
        enabled: always,
        build: |_| shell(ShellCommand::Hide),
    },
    CommandSpec {
        name: "app.hide-others",
        label: "Hide others",
        args: "",
        about: "Hide every other app.",
        enabled: always,
        build: |_| shell(ShellCommand::HideOthers),
    },
    CommandSpec {
        name: "app.minimize",
        label: "Minimize",
        args: "",
        about: "Minimize the window.",
        enabled: always,
        build: |_| shell(ShellCommand::Minimize),
    },
    CommandSpec {
        name: "window.close",
        label: "",
        args: "",
        about: "Close the window, as its close button does: for a layout that draws its own.",
        enabled: always,
        build: |_| shell(ShellCommand::Close),
    },
    CommandSpec {
        name: "window.zoom",
        label: "",
        args: "",
        about: "Zoom the window, as its zoom button or a title bar double click does.",
        enabled: always,
        build: |_| shell(ShellCommand::Zoom),
    },
    CommandSpec {
        name: "app.command-palette",
        label: "Command palette",
        args: "",
        about: "Open the command palette.",
        enabled: always,
        build: |_| shell(ShellCommand::CommandPalette),
    },
];

/// The space's canvases.
pub(super) const CANVAS: &[CommandSpec] = &[
    CommandSpec {
        name: "canvas.new",
        label: "New canvas",
        args: "{project?: id}",
        about: "Add an empty canvas to a project, the active canvas's by default, and show it.",
        enabled: idle,
        build: |args| {
            let project = maybe_text(args, "project").map(crate::ProjectId::new);
            act(Action::Canvas(CanvasAction::New(project)))
        },
    },
    CommandSpec {
        name: "canvas.duplicate",
        label: "Duplicate canvas",
        args: "{canvas?: id}",
        about: "Copy a canvas, the active one by default, and show the copy.",
        enabled: idle,
        build: |args| act(Action::Canvas(CanvasAction::Duplicate(canvas(args)))),
    },
    CommandSpec {
        name: "canvas.delete",
        label: "Delete canvas",
        args: "{canvas?: id}",
        about: "Remove a canvas, the active one by default, and trash its file.",
        enabled: idle,
        build: |args| act(Action::Canvas(CanvasAction::Delete(canvas(args)))),
    },
    CommandSpec {
        name: "canvas.switch",
        label: "",
        args: "{canvas: id}",
        about: "Show another canvas of the space.",
        enabled: idle,
        build: |args| {
            let id = CanvasId::new(text(args, "canvas")?);
            act(Action::Canvas(CanvasAction::Switch(id)))
        },
    },
    CommandSpec {
        name: "canvas.rename",
        label: "",
        args: "{name: text, canvas?: id}",
        about: "Give a canvas, the active one by default, another name.",
        enabled: idle,
        build: |args| {
            let name = text(args, "name")?;
            act(Action::Canvas(CanvasAction::Rename {
                canvas: canvas(args),
                name,
            }))
        },
    },
    CommandSpec {
        name: "canvas.begin-rename",
        label: "Rename canvas",
        args: "{canvas?: id}",
        about: "Start typing a new name for a canvas in its sidebar row.",
        enabled: idle,
        build: |args| act(Action::Canvas(CanvasAction::BeginRename(canvas(args)))),
    },
];

/// Sharing the active canvas.
pub(super) const SHARE: &[CommandSpec] = &[
    CommandSpec {
        name: "share.copy-link",
        label: "",
        args: "",
        about: "Copy the active canvas's link, publishing it first when it is not shared.",
        enabled: idle,
        build: |_| act(Action::Share(ShareAction::CopyLink)),
    },
    CommandSpec {
        name: "share.access",
        label: "",
        args: "{scope: edit|comment|view|none}",
        about: "Choose who the active canvas's link lets in; none stops sharing it.",
        enabled: idle,
        build: |args| {
            let scope = choice(
                args,
                "scope",
                &[
                    ("edit", Some(LinkScope::Edit)),
                    ("comment", Some(LinkScope::Comment)),
                    ("view", Some(LinkScope::View)),
                    ("none", None),
                ],
            )?;
            act(Action::Share(ShareAction::SetAccess(scope)))
        },
    },
    CommandSpec {
        name: "share.join",
        label: "",
        args: "{link: url, fresh?: bool}",
        about: "Share the active canvas through a link's live room, or a new canvas with fresh.",
        enabled: idle,
        build: |args| {
            let link = text(args, "link")?;
            if !link.contains("/c/") || !link.contains("#t=") {
                return Err(RunError::Args(
                    "`link` is a share link, `<server>/c/<doc>#t=<token>`".to_owned(),
                ));
            }
            let fresh = args.get("fresh").is_some() && switch(args, "fresh")?;
            Ok(Run::Api(ApiRun::Share { link, fresh }))
        },
    },
    CommandSpec {
        name: "share.stop",
        label: "",
        args: "",
        about: "Stop sharing the active canvas, keeping its document as it is.",
        enabled: idle,
        build: |_| Ok(Run::Api(ApiRun::Unshare)),
    },
];

/// Pressing a control the models show.
pub(super) const CONTROL: &[CommandSpec] = &[CommandSpec {
    name: "control.press",
    label: "",
    args: "{id: control name}",
    about: "Do what a click on the control with this name does.",
    enabled: always,
    build: |args| {
        let id = ControlId::from(text(args, "id")?);
        act(Action::PressControl(id))
    },
}];
