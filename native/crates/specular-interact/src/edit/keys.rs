//! The keys of the text editor: which key is which edit, and running it.
//!
//! These are the macOS text bindings the Electron editors have. Command is
//! Command or Control, as in the canvas binding table. Escape, undo and redo
//! are not here: they stay in that table, which ends the edit or steps the
//! editor's undo.

use specular_core::Modifiers;

use super::buffer::{Target, TextEdit};
use super::history::Change;
use super::lists;
use super::motion::{self, Motion, Seen};
use crate::{App, Effect, Key, KeyInput, bindings};

/// What Tab types in a Document outside a list.
const TAB: &str = "  ";

/// One thing a key does to the text being edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    /// Move the caret, extending the selection with Shift held.
    Move { motion: Motion, extend: bool },
    /// Delete the selection, or from the caret to where the motion goes.
    Delete(Motion),
    /// Select everything.
    SelectAll,
    /// Copy the selection to the clipboard.
    Copy,
    /// Copy the selection to the clipboard and delete it.
    Cut,
    /// Ask for the clipboard, which comes back as a paste.
    Paste,
    /// Enter.
    Newline,
    /// Tab.
    Indent,
    /// Shift+Tab.
    Outdent,
}

/// The motion an arrow, Home or End key makes with these modifiers.
fn motion_for(key: Key, modifiers: Modifiers) -> Option<Motion> {
    let cmd = modifiers.meta || modifiers.control;
    Some(match key {
        Key::ArrowLeft if cmd => Motion::LineStart,
        Key::ArrowLeft if modifiers.alt => Motion::WordLeft,
        Key::ArrowLeft => Motion::Left,
        Key::ArrowRight if cmd => Motion::LineEnd,
        Key::ArrowRight if modifiers.alt => Motion::WordRight,
        Key::ArrowRight => Motion::Right,
        Key::ArrowUp | Key::Home if cmd => Motion::DocStart,
        Key::ArrowDown | Key::End if cmd => Motion::DocEnd,
        Key::ArrowUp if modifiers.alt => Motion::ParagraphUp,
        Key::ArrowDown if modifiers.alt => Motion::ParagraphDown,
        Key::PageUp => Motion::PageUp,
        Key::PageDown => Motion::PageDown,
        Key::ArrowUp => Motion::Up,
        Key::ArrowDown => Motion::Down,
        Key::Home => Motion::LineStart,
        Key::End => Motion::LineEnd,
        Key::Escape
        | Key::Enter
        | Key::Tab
        | Key::Backspace
        | Key::Delete
        | Key::Space
        | Key::Char(_)
        | Key::Other => return None,
    })
}

/// The edit `input`'s key and modifiers ask for, if it is an editing key.
fn op_for(input: &KeyInput) -> Option<Op> {
    let modifiers = input.modifiers;
    let cmd = modifiers.meta || modifiers.control;
    if let Some(motion) = motion_for(input.key, modifiers) {
        return Some(Op::Move {
            motion,
            extend: modifiers.shift,
        });
    }
    Some(match input.key {
        Key::Backspace if cmd => Op::Delete(Motion::LineStart),
        Key::Backspace if modifiers.alt => Op::Delete(Motion::WordLeft),
        Key::Backspace => Op::Delete(Motion::Left),
        Key::Delete if cmd => Op::Delete(Motion::LineEnd),
        Key::Delete if modifiers.alt => Op::Delete(Motion::WordRight),
        Key::Delete => Op::Delete(Motion::Right),
        Key::Enter if !cmd => Op::Newline,
        Key::Tab if cmd => return None,
        Key::Tab if modifiers.shift => Op::Outdent,
        Key::Tab => Op::Indent,
        Key::Char('a') if cmd && !modifiers.shift => Op::SelectAll,
        Key::Char('c') if cmd && !modifiers.shift => Op::Copy,
        Key::Char('x') if cmd && !modifiers.shift => Op::Cut,
        Key::Char('v') if cmd => Op::Paste,
        Key::Escape
        | Key::Enter
        | Key::Space
        | Key::ArrowLeft
        | Key::ArrowRight
        | Key::ArrowUp
        | Key::ArrowDown
        | Key::Home
        | Key::End
        | Key::PageUp
        | Key::PageDown
        | Key::Char(_)
        | Key::Other => return None,
    })
}

/// Runs `op` on `edit`. Returns whether the text changed.
fn run(edit: &mut TextEdit, op: Op, seen: Seen<'_>, effects: &mut Vec<Effect>) -> bool {
    let (lists, tabs) = match edit.target {
        Target::Text => (true, false),
        Target::Note => (true, true),
        Target::Label | Target::Title | Target::EdgeLabel | Target::Comment => (false, false),
    };
    match op {
        Op::Move { motion, extend } => {
            motion::apply(edit, motion, extend, seen);
            false
        }
        Op::Delete(Motion::Left) if lists && lists::unbullet(edit) => true,
        Op::Delete(motion) => motion::delete(edit, motion, seen),
        Op::SelectAll => {
            edit.select(0..edit.text.len());
            false
        }
        Op::Copy | Op::Cut => {
            if edit.selection().is_empty() {
                return false;
            }
            effects.push(Effect::WriteClipboard(edit.selected().to_owned()));
            op == Op::Cut && edit.delete(edit.selection(), Change::Single)
        }
        Op::Paste => {
            effects.push(Effect::ReadClipboard);
            false
        }
        Op::Newline if lists => lists::newline(edit),
        Op::Newline => edit.insert("\n", Change::Single),
        // Outside a list, Tab in a Document is the indent it types.
        Op::Indent => (lists && lists::indent(edit)) || (tabs && edit.insert(TAB, Change::Typing)),
        Op::Outdent => lists && lists::outdent(edit),
    }
}

/// The text a key press types, with anything that is not a character to
/// keep taken out.
fn typed(input: &KeyInput) -> Option<String> {
    let cmd = input.modifiers.meta || input.modifiers.control;
    let text: String = (input.text.as_deref().filter(|_| !cmd)?.chars())
        .filter(|character| !character.is_control())
        .collect();
    (!text.is_empty()).then_some(text)
}

/// Offers a key to the text being edited. Returns `false` for a key the
/// canvas binding table should have instead: Escape, undo and redo.
pub(crate) fn on_key(app: &mut App, input: &KeyInput, effects: &mut Vec<Effect>) -> bool {
    let Some(mut edit) = app.session.editing.take() else {
        return false;
    };
    let op = op_for(input);
    // While the input method is composing, the keys are its own. Escape
    // still ends the edit.
    let composing = edit.composition.is_some() && input.key != Key::Escape;
    let mut changed = false;
    let taken = if composing {
        true
    } else if let Some(op) = op {
        if input.pressed {
            let layout = super::layout_of(app, &edit).unwrap_or_default();
            let seen = Seen {
                layout: &layout,
                page: super::page_height(app, &edit),
            };
            changed = run(&mut edit, op, seen, effects);
        }
        true
    } else {
        false
    };
    app.session.editing = Some(edit);
    if !taken {
        if bindings::binding_for(app, input).is_some() {
            return false;
        }
        if input.pressed
            && let Some(text) = typed(input)
            && let Some(edit) = &mut app.session.editing
        {
            changed = edit.insert(&text, Change::Typing);
        }
    }
    if changed {
        super::refit(app);
    }
    true
}
