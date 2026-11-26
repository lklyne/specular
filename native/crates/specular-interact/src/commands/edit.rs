//! The Edit menu's commands, the selection, and the text formats.

use super::CommandSpec;
use super::args::{act, always, can, idle, items, number};
use crate::{Action, Format};

/// Undo, the clipboard, the selection.
pub(super) const EDIT: &[CommandSpec] = &[
    CommandSpec {
        name: "edit.cancel",
        label: "",
        args: "",
        about: "Back out one stage, as Escape does: a draft, a drag, a tool, a page, the selection.",
        enabled: always,
        build: |_| act(Action::Cancel),
    },
    CommandSpec {
        name: "edit.undo",
        label: "Undo",
        args: "",
        about: "Undo the latest step, or the latest step of the text being edited.",
        enabled: |app| can(app, &Action::Undo),
        build: |_| act(Action::Undo),
    },
    CommandSpec {
        name: "edit.redo",
        label: "Redo",
        args: "",
        about: "Redo the latest undone step.",
        enabled: |app| can(app, &Action::Redo),
        build: |_| act(Action::Redo),
    },
    CommandSpec {
        name: "edit.cut",
        label: "Cut",
        args: "",
        about: "Put the selection on the clipboard and remove it.",
        enabled: |app| can(app, &Action::Cut),
        build: |_| act(Action::Cut),
    },
    CommandSpec {
        name: "edit.copy",
        label: "Copy",
        args: "",
        about: "Put the selection on the clipboard.",
        enabled: |app| can(app, &Action::Copy),
        build: |_| act(Action::Copy),
    },
    CommandSpec {
        name: "edit.paste",
        label: "Paste",
        args: "",
        about: "Paste the clipboard at the pointer.",
        enabled: idle,
        build: |_| act(Action::Paste),
    },
    CommandSpec {
        name: "edit.duplicate",
        label: "Duplicate",
        args: "",
        about: "Copy the selection into free space beside it and select the copies.",
        enabled: |app| can(app, &Action::Duplicate),
        build: |_| act(Action::Duplicate),
    },
    CommandSpec {
        name: "edit.delete",
        label: "Delete",
        args: "",
        about: "Remove the selection, or the focused comment.",
        enabled: |app| can(app, &Action::Delete),
        build: |_| act(Action::Delete),
    },
    CommandSpec {
        name: "edit.select-all",
        label: "Select all",
        args: "",
        about: "Select everything that is not inside a group.",
        enabled: |app| can(app, &Action::SelectAll),
        build: |_| act(Action::SelectAll),
    },
    CommandSpec {
        name: "edit.select",
        label: "",
        args: "{ids?: [entity id], edges?: [edge id]}",
        about: "Replace the selection; ids that name nothing are dropped.",
        enabled: idle,
        build: |args| act(Action::Select(items(args)?)),
    },
    CommandSpec {
        name: "edit.nudge",
        label: "",
        args: "{dx: number, dy: number}",
        about: "Move the selection by exactly this many canvas units.",
        enabled: idle,
        build: |args| {
            act(Action::Nudge {
                dx: number(args, "dx")?,
                dy: number(args, "dy")?,
            })
        },
    },
];

/// Whether there is text being edited to format.
fn editing(app: &crate::App) -> bool {
    can(app, &Action::Format(Format::Bold))
}

/// The markdown formats of the text being edited.
pub(super) const TEXT: &[CommandSpec] = &[
    CommandSpec {
        name: "text.bold",
        label: "Bold",
        args: "",
        about: "Toggle bold on the selected text.",
        enabled: editing,
        build: |_| act(Action::Format(Format::Bold)),
    },
    CommandSpec {
        name: "text.italic",
        label: "Italic",
        args: "",
        about: "Toggle italic on the selected text.",
        enabled: editing,
        build: |_| act(Action::Format(Format::Italic)),
    },
    CommandSpec {
        name: "text.code",
        label: "Code",
        args: "",
        about: "Toggle inline code on the selected text.",
        enabled: editing,
        build: |_| act(Action::Format(Format::Code)),
    },
    CommandSpec {
        name: "text.strikethrough",
        label: "Strikethrough",
        args: "",
        about: "Toggle strikethrough on the selected text.",
        enabled: editing,
        build: |_| act(Action::Format(Format::Strike)),
    },
    CommandSpec {
        name: "text.bullet-list",
        label: "Bullet list",
        args: "",
        about: "Make the selected lines a bullet list, or plain lines again.",
        enabled: editing,
        build: |_| act(Action::Format(Format::BulletList)),
    },
    CommandSpec {
        name: "text.numbered-list",
        label: "Numbered list",
        args: "",
        about: "Make the selected lines a numbered list, or plain lines again.",
        enabled: editing,
        build: |_| act(Action::Format(Format::NumberedList)),
    },
    CommandSpec {
        name: "text.task-list",
        label: "Task list",
        args: "",
        about: "Make the selected lines a task list, or plain lines again.",
        enabled: editing,
        build: |_| act(Action::Format(Format::TaskList)),
    },
    CommandSpec {
        name: "text.body",
        label: "Body text",
        args: "",
        about: "Make the selected lines body text.",
        enabled: editing,
        build: |_| act(Action::Format(Format::Heading(0))),
    },
    CommandSpec {
        name: "text.heading-1",
        label: "Heading 1",
        args: "",
        about: "Make the selected lines a level 1 heading.",
        enabled: editing,
        build: |_| act(Action::Format(Format::Heading(1))),
    },
    CommandSpec {
        name: "text.heading-2",
        label: "Heading 2",
        args: "",
        about: "Make the selected lines a level 2 heading.",
        enabled: editing,
        build: |_| act(Action::Format(Format::Heading(2))),
    },
    CommandSpec {
        name: "text.heading-3",
        label: "Heading 3",
        args: "",
        about: "Make the selected lines a level 3 heading.",
        enabled: editing,
        build: |_| act(Action::Format(Format::Heading(3))),
    },
    CommandSpec {
        name: "text.heading-4",
        label: "Heading 4",
        args: "",
        about: "Make the selected lines a level 4 heading.",
        enabled: editing,
        build: |_| act(Action::Format(Format::Heading(4))),
    },
    CommandSpec {
        name: "text.heading-5",
        label: "Heading 5",
        args: "",
        about: "Make the selected lines a level 5 heading.",
        enabled: editing,
        build: |_| act(Action::Format(Format::Heading(5))),
    },
    CommandSpec {
        name: "text.heading-6",
        label: "Heading 6",
        args: "",
        about: "Make the selected lines a level 6 heading.",
        enabled: editing,
        build: |_| act(Action::Format(Format::Heading(6))),
    },
];
