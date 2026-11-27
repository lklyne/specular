//! A key through the table: its binding, or the page's edit command.

use specular_core::{PageEdit, PointerEventKind};

use super::{BINDINGS, Binding};
use crate::commands::Run;
use crate::update::run_action;
use crate::{
    Action, App, Chord, Effect, Event, Focus, KeyInput, PointerInput, edit, gesture, page_input,
};

/// The Edit menu command `action` is inside a page, if it is one.
pub(crate) const fn page_edit(action: &Action) -> Option<PageEdit> {
    match action {
        Action::Undo => Some(PageEdit::Undo),
        Action::Redo => Some(PageEdit::Redo),
        Action::Cut => Some(PageEdit::Cut),
        Action::Copy => Some(PageEdit::Copy),
        Action::Paste => Some(PageEdit::Paste),
        Action::SelectAll => Some(PageEdit::SelectAll),
        _ => None,
    }
}

/// A browser's Edit menu has one item the canvas has no meaning for.
const PASTE_AND_MATCH_STYLE: Chord = Chord::char('v').cmd().shift().alt();

/// The Edit menu command a press of `input` is the key equivalent of inside
/// a page: the key of that item's row in [`BINDINGS`], with Command and not
/// Control.
pub(crate) fn page_edit_for(input: &KeyInput) -> Option<PageEdit> {
    if !input.modifiers.meta || input.modifiers.control {
        return None;
    }
    let chord = Chord::of(input);
    if chord == PASTE_AND_MATCH_STYLE {
        return Some(PageEdit::PasteAndMatchStyle);
    }
    BINDINGS
        .iter()
        .filter(|binding| binding.chord == chord)
        .find_map(|binding| page_edit(&binding.action))
}

/// The binding `input`'s key and modifiers fire in `app` as it is now.
pub(crate) fn binding_for(app: &App, input: &KeyInput) -> Option<&'static Binding> {
    let chord = Chord::of(input);
    BINDINGS
        .iter()
        .find(|binding| binding.chord == chord && binding.context.holds(app))
}

pub(crate) fn on_key(app: &mut App, input: &KeyInput, effects: &mut Vec<Effect>) {
    app.session.modifiers = input.modifiers;
    // The way back from a layout that hid what was needed to undo it:
    // nothing, a field or a page included, has this key first.
    if crate::chrome::is_reset(input) {
        if input.pressed && !input.repeat {
            run_action(app, Action::Layout(crate::LayoutAction::Reset), effects);
        }
        return;
    }
    // A drag reads Shift, Option and Command, so one that changes while the
    // pointer is still takes effect at once.
    if let (Some(_), Some(screen)) = (&app.session.gesture, app.session.pointer) {
        let held = PointerInput {
            kind: PointerEventKind::Move,
            screen,
            modifiers: input.modifiers,
        };
        gesture::drag(app, &held);
    }
    // The text being edited has the keys first. It leaves Escape, undo and
    // redo to the table.
    if edit::on_key(app, input, effects) {
        return;
    }
    // An open dialog, popup or drawer is above everything but the field.
    if input.key == crate::Key::Escape && input.pressed && crate::overlays::escape(app) {
        return;
    }
    if let Some(row) = app.keymap.row_for(app, Chord::of(input)) {
        // The person's key, or the table's taken away: what is under it
        // has the key.
        let Some(run) = row.run.clone() else {
            unbound(app, input, effects);
            return;
        };
        if input.pressed && !input.repeat {
            run_keyed(app, run, effects);
        }
        return;
    }
    match binding_for(app, input) {
        Some(binding) => {
            // The release of a bound key goes nowhere either.
            if input.pressed && (binding.repeats || !input.repeat) {
                run_action(app, binding.action.clone(), effects);
            }
        }
        None => unbound(app, input, effects),
    }
}

/// A key no binding takes: an entered page's, else a mod command's.
fn unbound(app: &mut App, input: &KeyInput, effects: &mut Vec<Effect>) {
    match &app.session.focus {
        Focus::Page(page) => page_input::forward_key(page, input, effects),
        Focus::Canvas => {
            if let Some(action) = crate::mods::bound_to(app, Chord::of(input))
                && input.pressed
                && !input.repeat
            {
                run_action(app, action, effects);
            }
        }
    }
}

/// Runs what a key of `keys.json` names: an action here, the window's own
/// command there, an API change as the API runs it.
fn run_keyed(app: &mut App, run: Run, effects: &mut Vec<Effect>) {
    match run.into_event() {
        Ok(Event::Action(action)) => run_action(app, action, effects),
        Ok(event) => effects.extend(crate::update(app, event)),
        Err(command) => effects.push(Effect::RunShell(command)),
    }
}
