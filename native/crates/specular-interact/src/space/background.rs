//! Running a change on a background canvas without showing it: how a
//! `--tab` write reaches a canvas the user is not looking at.

use std::mem;

use super::{CanvasId, CanvasState, CanvasView, Parked};
use crate::{App, Effect, Session};

/// Runs `run` with the background canvas `id` standing where the active
/// one does, then puts everything back. The user's session is set aside
/// whole for the run, so their selection, their text edit and their drag
/// are out of its reach, and the canvas gets its own selection and undo
/// step.
///
/// The canvas has no page hosts, images or Documents loaded, so of the
/// effects `run` returns only the reply is kept. Whatever it changed is
/// loaded when the canvas is switched to. If it made an undo step the
/// canvas is written. Returns `false` when `id` is the active canvas or
/// names none; `run` is not called then.
pub(crate) fn in_background(
    app: &mut App,
    id: &CanvasId,
    effects: &mut Vec<Effect>,
    run: impl FnOnce(&mut App, &mut Vec<Effect>),
) -> bool {
    let Some(index) = app.space.index_of(id) else {
        return false;
    };
    let CanvasState::Parked(slot) = &mut app.space.canvases[index].state else {
        return false;
    };
    let Parked {
        document,
        history,
        view,
    } = mem::take(&mut **slot);

    let theirs = Session {
        camera: view.camera,
        selection: view.selection,
        loaded_fits: view.loaded_fits,
        viewport: app.session.viewport,
        now_ms: app.session.now_ms,
        id_state: app.session.id_state,
        ..Session::default()
    };
    let user_session = mem::replace(&mut app.session, theirs);
    let user_document = mem::replace(&mut app.document, document);
    let user_history = mem::replace(&mut app.history, history);

    let revision = app.history.revision();
    let mut returned = Vec::new();
    run(app, &mut returned);
    if app.history.is_open() {
        app.history.settle(app.session.selection.clone());
    }
    let changed = app.history.revision() != revision;

    let theirs = mem::replace(&mut app.session, user_session);
    // Ids drawn during the run are spent.
    app.session.id_state = theirs.id_state;
    let after = Parked {
        document: mem::replace(&mut app.document, user_document),
        history: mem::replace(&mut app.history, user_history),
        view: CanvasView {
            camera: theirs.camera,
            selection: theirs.selection,
            loaded_fits: theirs.loaded_fits,
            fitted: view.fitted,
        },
    };
    if let CanvasState::Parked(slot) = &mut app.space.canvases[index].state {
        **slot = after;
    }

    if changed {
        effects.push(Effect::WriteCanvas(id.clone()));
    }
    effects
        .extend((returned.into_iter()).filter(|effect| matches!(effect, Effect::ApiReply { .. })));
    true
}
