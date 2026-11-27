//! What the window draws: hiding and showing its regions and controls over
//! the layout file, and going back to the shipped layout.

use serde_json::Value;

use super::args::{act, always, text};
use super::{CommandSpec, Run, RunError};
use crate::{Action, LayoutAction};

/// The `id` argument: a region's name or a control's id.
fn id(args: &Value) -> Result<String, RunError> {
    let id = text(args, "id")?;
    if id.is_empty() {
        return Err(RunError::Args(
            "`id` names a region or a control".to_owned(),
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
        args: "{id: tabs|toolbar|dock|sidebar|chat|menu|share|toast|palette|welcome|<control id>}",
        about: "Hide a region of the window or a control if it is shown, show it if hidden.",
        enabled: always,
        build: |args| layout(LayoutAction::Toggle(id(args)?)),
    },
    CommandSpec {
        name: "ui.show",
        label: "",
        args: "{id: tabs|toolbar|dock|sidebar|chat|menu|share|toast|palette|welcome|<control id>}",
        about: "Show a region of the window or a control.",
        enabled: always,
        build: |args| layout(LayoutAction::Show(id(args)?)),
    },
    CommandSpec {
        name: "ui.hide",
        label: "",
        args: "{id: tabs|toolbar|dock|sidebar|chat|menu|share|toast|palette|welcome|<control id>}",
        about: "Hide a region of the window or a control, giving its room to the canvas.",
        enabled: always,
        build: |args| layout(LayoutAction::Hide(id(args)?)),
    },
    CommandSpec {
        name: "ui.reset",
        label: "",
        args: "",
        about: "Set aside what was hidden and shown, and go back to the shipped layout.",
        enabled: always,
        build: |_| layout(LayoutAction::Reset),
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
