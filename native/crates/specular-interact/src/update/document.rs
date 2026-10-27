//! Document steps: one undo step, an undo or a redo, a document opened, and what each leaves dangling.

use specular_doc::{Command, CommandError, Document, EntityId, ItemId};

use crate::focus::set_focus;
use crate::{App, Effect, Selection, comment, edit, gesture, pages};
use crate::{images, notes};

/// Runs `command` as one undo step, then brings the page hosts and the
/// session back in step with the document.
pub(crate) fn document_step(app: &mut App, command: Command, effects: &mut Vec<Effect>) {
    let before = pages::snapshot(app);
    gesture::apply_step(app, command);
    drop_dangling(app, effects);
    pages::reconcile(&before, app, effects);
}

/// Runs an undo or a redo, unless a drag is in flight, puts back the
/// selection the step carries, and brings the page hosts and the session
/// back in step with the document.
pub(super) fn step_history(
    app: &mut App,
    effects: &mut Vec<Effect>,
    step: impl FnOnce(&mut App) -> Result<Option<Selection>, CommandError>,
) {
    if app.session.gesture.is_some() {
        return;
    }
    let before = pages::snapshot(app);
    let notes_before = edit::note::held(app);
    match step(app) {
        Ok(Some(selection)) => app.session.selection = selection,
        Ok(None) => return,
        Err(error) => tracing::warn!("history step dropped: {error}"),
    }
    edit::note::write_stepped(app, &notes_before, effects);
    drop_dangling(app, effects);
    pages::reconcile(&before, app, effects);
}

pub(crate) fn open_document(app: &mut App, document: Document, effects: &mut Vec<Effect>) {
    let before = pages::snapshot(app);
    app.session.gesture = None;
    app.session.entered_group = None;
    edit::discard(app, effects);
    comment::forget(app);
    app.document = document;
    edit::fit_all(app);
    app.history.clear();
    drop_dangling(app, effects);
    pages::reconcile(&before, app, effects);
    images::reopen(app, effects);
    notes::reopen(app, effects);
}

/// Forgets selection, hover and focus that name something the document no
/// longer holds.
pub(crate) fn drop_dangling(app: &mut App, effects: &mut Vec<Effect>) {
    let document = &app.document;
    app.session.selection.retain(|item| match item {
        ItemId::Entity(id) => document.entity(id).is_some(),
        ItemId::Edge(id) => document.edge(id).is_some(),
    });
    let gone = |page: Option<&EntityId>| page.is_some_and(|id| app.page_placement(id).is_none());
    let (pointer_gone, focus_gone) = (
        gone(app.session.pointer_page.as_ref()),
        gone(app.session.focus.page()),
    );
    if pointer_gone {
        app.session.pointer_page = None;
    }
    if (app.session.hover.as_ref()).is_some_and(|id| app.document.entity(id).is_none()) {
        app.session.hover = None;
    }
    if focus_gone {
        set_focus(app, None, effects);
    }
    let document = &app.document;
    (app.session.pages).retain(|page| document.entity(page).is_some());
    (app.session.sync).retain(|page| document.entity(page).is_some());
}
