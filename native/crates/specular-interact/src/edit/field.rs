//! A text field of a panel edited in place: the address of a page, a custom
//! size.
//!
//! The field is chrome, which the shell draws. With no shell drawing it (a
//! headless run) its text is an edit session keyed by an [`EntityId`] holding
//! the control's name, which no entity has, so the editor's caret, selection,
//! clipboard and input method work on it as they do anywhere.
//!
//! Enter keeps what was typed, and so does a press anywhere else, as a
//! field's blur does. Escape puts the old value back. Keeping a value runs
//! the action its [`FieldSubmit`](crate::FieldSubmit) makes of the text;
//! text that asks for nothing leaves the field as the model has it.

use specular_doc::{EntityId, Rect};

use super::buffer::{Origin, Target, TextEdit};
use crate::focus::set_focus;
use crate::panel::{ControlId, field_named};
use crate::update::run_action;
use crate::{App, Effect, gesture};

fn control_of(edit: &TextEdit) -> ControlId {
    ControlId::from(edit.entity.as_str().to_owned())
}

/// The field being edited, if one is.
fn editing(app: &App) -> Option<&TextEdit> {
    app.session.editing.as_ref().filter(|edit| edit.is_field())
}

/// Starts editing the field named `id` with all of its value selected.
/// Whatever was being edited ends first, and an entered page gives up the
/// keys. Does nothing for a field the model does not show.
pub(crate) fn begin(app: &mut App, id: &ControlId, effects: &mut Vec<Effect>) {
    if editing(app).is_some_and(|edit| control_of(edit) == *id) {
        return;
    }
    super::end(app, effects);
    let Some(field) = field_named(app, id) else {
        return;
    };
    set_focus(app, None, effects);
    let origin = Origin {
        text: field.value.clone(),
        rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        created: false,
    };
    let key = EntityId::from(id.as_str());
    let mut edit = TextEdit::new(key, Target::Field, &field.value, origin);
    edit.active_ms = app.session.now_ms;
    app.session.editing = Some(edit);
    effects.push(Effect::SetImeAllowed(true));
    super::place_candidates(app, effects);
}

/// Puts the keys in the field named `id` with its value selected: a shell
/// that draws the chrome is asked to give its own the keys, and with none
/// the field is edited here.
pub(crate) fn focus(app: &mut App, id: &ControlId, effects: &mut Vec<Effect>) {
    if !app.session.panel.shell_chrome {
        begin(app, id, effects);
    } else if field_named(app, id).is_some() {
        super::end(app, effects);
        effects.push(Effect::EditField(id.clone()));
    }
}

/// Ends the edit keeping what was typed: the action its text asks for is
/// run, unless the text is as it was.
pub(super) fn end(app: &mut App, edit: &TextEdit, effects: &mut Vec<Effect>) {
    if edit.text.trim() == edit.origin.text.trim() {
        return;
    }
    let Some(field) = field_named(app, &control_of(edit)) else {
        return;
    };
    if let Some(action) = field.submit.action(&edit.text) {
        run_action(app, action, effects);
    }
}

/// Escape in a field: the typed text is dropped and the old value shows
/// again. Returns whether a field was being edited.
pub(crate) fn cancel(app: &mut App, effects: &mut Vec<Effect>) -> bool {
    if editing(app).is_none() {
        return false;
    }
    gesture::cancel(app, effects);
    app.session.editing = None;
    effects.push(Effect::SetImeAllowed(false));
    true
}
