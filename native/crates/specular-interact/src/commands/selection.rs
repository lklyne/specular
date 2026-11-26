//! The Arrange and Comment menus' commands: the stack order, groups, laying
//! out, and comments.

use serde_json::Value;
use specular_doc::AnnotationId;

use super::args::{act, can, choice, idle, maybe_text, number, text};
use super::{CommandSpec, Run, RunError};
use crate::{Action, ArrangeMode, LayoutAxis};

fn comment(args: &Value) -> Option<AnnotationId> {
    maybe_text(args, "id").map(AnnotationId::from)
}

fn lay_out(args: &Value) -> Result<Run, RunError> {
    let mode = choice(
        args,
        "mode",
        &[
            ("row", ArrangeMode::Row),
            ("column", ArrangeMode::Column),
            ("grid", ArrangeMode::Grid),
        ],
    )?;
    act(Action::Arrange(mode))
}

fn group_layout(args: &Value) -> Result<Run, RunError> {
    let axis = choice(
        args,
        "axis",
        &[
            ("row", Some(LayoutAxis::X)),
            ("column", Some(LayoutAxis::Y)),
            ("none", None),
        ],
    )?;
    act(Action::GroupLayout(axis))
}

/// The stack order, groups and laying out.
pub(super) const ARRANGE: &[CommandSpec] = &[
    CommandSpec {
        name: "arrange.bring-forward",
        label: "Bring forward",
        args: "",
        about: "Move the selection one slot forward in the stack order.",
        enabled: |app| can(app, &Action::BringForward),
        build: |_| act(Action::BringForward),
    },
    CommandSpec {
        name: "arrange.send-backward",
        label: "Send backward",
        args: "",
        about: "Move the selection one slot backward in the stack order.",
        enabled: |app| can(app, &Action::SendBackward),
        build: |_| act(Action::SendBackward),
    },
    CommandSpec {
        name: "arrange.bring-to-front",
        label: "Bring to front",
        args: "",
        about: "Move the selection in front of everything.",
        enabled: |app| can(app, &Action::BringToFront),
        build: |_| act(Action::BringToFront),
    },
    CommandSpec {
        name: "arrange.send-to-back",
        label: "Send to back",
        args: "",
        about: "Move the selection behind everything.",
        enabled: |app| can(app, &Action::SendToBack),
        build: |_| act(Action::SendToBack),
    },
    CommandSpec {
        name: "arrange.group",
        label: "Group",
        args: "",
        about: "Wrap the selected items in a new group and select it.",
        enabled: |app| can(app, &Action::Group),
        build: |_| act(Action::Group),
    },
    CommandSpec {
        name: "arrange.ungroup",
        label: "Ungroup",
        args: "",
        about: "Take the selected group apart and select what was inside it.",
        enabled: |app| can(app, &Action::Ungroup),
        build: |_| act(Action::Ungroup),
    },
    CommandSpec {
        name: "arrange.auto-layout",
        label: "Auto-layout",
        args: "",
        about: "Make the selection an auto-layout group that packs its members.",
        enabled: |app| can(app, &Action::AutoLayout),
        build: |_| act(Action::AutoLayout),
    },
    CommandSpec {
        name: "arrange.lay-out",
        label: "",
        args: "{mode: row|column|grid}",
        about: "Lay the selected items out in a row, a column or a grid, evening the gaps.",
        enabled: |app| can(app, &Action::Arrange(ArrangeMode::Row)),
        build: lay_out,
    },
    CommandSpec {
        name: "arrange.group-layout",
        label: "",
        args: "{axis: row|column|none}",
        about: "Pack the selected group's members along an axis, or let them sit where they are.",
        enabled: |app| can(app, &Action::GroupLayout(None)),
        build: group_layout,
    },
    CommandSpec {
        name: "arrange.group-gap",
        label: "",
        args: "{gap: number}",
        about: "Set the gap the selected auto-layout group packs with, in canvas units.",
        enabled: |app| can(app, &Action::GroupGap(0.0)),
        build: |args| act(Action::GroupGap(number(args, "gap")?)),
    },
];

/// Comments.
pub(super) const COMMENT: &[CommandSpec] = &[
    CommandSpec {
        name: "comment.annotate-selection",
        label: "Annotate selection",
        args: "",
        about: "Open a comment draft on the region the selected entities span.",
        enabled: |app| can(app, &Action::AnnotateSelection),
        build: |_| act(Action::AnnotateSelection),
    },
    CommandSpec {
        name: "comment.resolve",
        label: "Resolve comment",
        args: "{id?: comment id}",
        about: "Mark a comment resolved, or every comment on the focused one's mark.",
        enabled: idle,
        build: |args| act(Action::ResolveComment(comment(args))),
    },
    CommandSpec {
        name: "comment.delete",
        label: "Delete comment",
        args: "{id?: comment id}",
        about: "Remove a comment, or every comment on the focused one's mark.",
        enabled: idle,
        build: |args| act(Action::DeleteComment(comment(args))),
    },
    CommandSpec {
        name: "comment.focus",
        label: "",
        args: "{id?: comment id}",
        about: "Give a comment the focus, or with no id let go of it.",
        enabled: idle,
        build: |args| act(Action::FocusComment(comment(args))),
    },
    CommandSpec {
        name: "comment.reveal",
        label: "",
        args: "{id: comment id}",
        about: "Give a comment the focus and bring what it is on into view.",
        enabled: idle,
        build: |args| act(Action::RevealComment(text(args, "id")?.into())),
    },
];
