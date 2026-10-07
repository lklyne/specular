//! Input-method composition in the editor.
//!
//! The text being composed sits in the working text, and
//! [`TextEdit::composition`] marks it. A whole composition, from its first
//! marked character to its commit, is one step of the editor's undo, and a
//! cancelled one leaves no step.

use std::ops::Range;

use specular_core::ImeEvent;

use super::buffer::TextEdit;
use super::history::Change;
use super::segment;

/// A range in UTF-16 code units, as the input method counts, in bytes.
fn bytes(text: &str, units: &Range<u32>) -> Range<usize> {
    let start = segment::from_utf16(text, units.start);
    start..segment::from_utf16(text, units.end).max(start)
}

/// What the next composed or committed text replaces: the composition so
/// far, else the range the input method named, else the selection.
fn replaced(edit: &TextEdit, replacement: Option<&Range<u32>>) -> Range<usize> {
    match (&edit.composition, replacement) {
        (Some(composing), _) => composing.clone(),
        (None, Some(units)) => bytes(&edit.text, units),
        (None, None) => edit.selection(),
    }
}

/// Applies one input-method event. Returns whether the text changed.
pub(crate) fn apply(edit: &mut TextEdit, event: &ImeEvent) -> bool {
    let before = edit.text.clone();
    match event {
        ImeEvent::SetComposition {
            text,
            selection,
            replacement,
        } => {
            let range = replaced(edit, replacement.as_ref());
            if edit.composition.is_none() {
                edit.checkpoint();
            }
            let start = range.start;
            edit.splice(range, text);
            let marked = bytes(text, selection);
            edit.anchor = start + marked.start;
            edit.caret = start + marked.end;
            edit.composition = (!text.is_empty()).then(|| start..start + text.len());
            if edit.composition.is_none() {
                edit.history.forget_if_unchanged(&edit.text);
            }
        }
        ImeEvent::Commit { text, replacement } => {
            let range = replaced(edit, replacement.as_ref());
            if edit.composition.take().is_some() {
                // The step was recorded when the composition began.
                edit.splice(range, text);
                edit.history.forget_if_unchanged(&edit.text);
            } else {
                edit.change(Change::Typing, |edit| edit.splice(range, text));
            }
        }
        ImeEvent::FinishComposing { keep_selection } => {
            if let Some(composing) = edit.composition.take()
                && !keep_selection
            {
                edit.caret = composing.end;
                edit.anchor = composing.end;
            }
        }
        ImeEvent::Cancel => {
            if let Some(composing) = edit.composition.take() {
                edit.splice(composing, "");
                edit.history.forget_if_unchanged(&edit.text);
            }
        }
    }
    edit.history.seal();
    edit.text != before
}
