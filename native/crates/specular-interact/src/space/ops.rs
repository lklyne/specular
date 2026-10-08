//! What can be done to the canvases of the space: open a space, switch,
//! new, rename, duplicate and delete. Each changes the list in memory and
//! returns the effects that put the same change on disk.
//!
//! A background canvas has no page hosts. Switching closes every page of
//! the canvas being left and creates every page of the one being entered,
//! so a canvas costs nothing while it is not shown and a page reloads when
//! its canvas comes back.

use std::mem;

use specular_doc::Document;

use super::{
    Canvas, CanvasId, CanvasState, CanvasView, DEFAULT_CANVAS_NAME, OpenedSpace, Parked, Space,
    canvas_file_name, names,
};
use crate::focus::set_focus;
use crate::pages::{self, HostedPage};
use crate::update::{drop_dangling, open_document};
use crate::{App, CanvasAction, Effect, Session, comment, edit, images, notes};

/// Runs a change to the space's canvases, unless a drag is in flight: the
/// drag owns the document it started on. One that is refused, a rename to
/// a name that is taken, leaves everything as it was.
pub(crate) fn act(app: &mut App, action: CanvasAction, effects: &mut Vec<Effect>) {
    if app.session.gesture.is_some() {
        return;
    }
    // With no canvas named, the active one.
    let target = |app: &App, canvas: Option<CanvasId>| {
        canvas.unwrap_or_else(|| app.space.active().id.clone())
    };
    let done = match action {
        CanvasAction::Switch(canvas) => {
            switch(app, &canvas, effects);
            Ok(())
        }
        CanvasAction::New => create(app, None, true, effects).map(|_| ()),
        CanvasAction::Rename { canvas, name } => {
            let canvas = target(app, canvas);
            rename(app, &canvas, &name, effects)
        }
        CanvasAction::Duplicate(canvas) => {
            let canvas = target(app, canvas);
            duplicate(app, &canvas, effects);
            Ok(())
        }
        CanvasAction::Delete(canvas) => {
            let canvas = target(app, canvas);
            delete(app, &canvas, effects);
            Ok(())
        }
    };
    if let Err(reason) = done {
        tracing::info!("canvas left as it was: {reason}");
    }
}

/// Replaces the space with one just read from disk and shows its active
/// canvas. Nothing of the old space carries over.
pub(crate) fn open(app: &mut App, opened: OpenedSpace, effects: &mut Vec<Effect>) {
    let before = pages::snapshot(&app.document);
    app.session.gesture = None;
    edit::discard(app, effects);
    comment::forget(app);
    set_focus(app, None, effects);

    let OpenedSpace {
        folder,
        canvases,
        active,
    } = opened;
    let mut entries: Vec<Canvas> = canvases
        .into_iter()
        .map(|canvas| Canvas {
            id: canvas.id,
            name: canvas.name,
            file: canvas.file,
            state: parked(canvas.document, canvas.camera, false),
        })
        .collect();
    let made = entries.is_empty().then(|| {
        let first = new_canvas(
            &mut app.session,
            &entries,
            DEFAULT_CANVAS_NAME.to_owned(),
            Parked::default(),
        );
        let id = first.id.clone();
        entries.push(first);
        id
    });
    let index = active
        .and_then(|id| entries.iter().position(|canvas| canvas.id == id))
        .unwrap_or(0);
    let entering = mem::replace(&mut entries[index].state, CanvasState::Active);
    app.space = Space {
        folder,
        canvases: entries,
        active: index,
        switches: app.space.switches + 1,
    };
    app.history = specular_doc::History::default();
    if let CanvasState::Parked(entering) = entering {
        enter(app, *entering, &before, effects);
    }
    if let Some(id) = made {
        effects.push(Effect::WriteCanvas(id));
        effects.push(Effect::SaveSpaceMeta);
    }
}

/// Shows the canvas `id`. Returns whether it is now the active one.
pub(crate) fn switch(app: &mut App, id: &CanvasId, effects: &mut Vec<Effect>) -> bool {
    let Some(index) = app.space.index_of(id) else {
        return false;
    };
    if index != app.space.active {
        activate(app, index, effects);
        effects.push(Effect::SaveSpaceMeta);
    }
    true
}

/// Adds an empty canvas at the end of the list. With no `name` it is the
/// next `Canvas N`. A name is refused when it is empty or taken: a tab ref
/// resolves by exact name, so a second canvas with one would make every
/// ref to it ambiguous.
pub(crate) fn create(
    app: &mut App,
    name: Option<&str>,
    activate_it: bool,
    effects: &mut Vec<Effect>,
) -> Result<CanvasId, String> {
    let name = match name.map(str::trim) {
        None => names::next_default(&app.space.canvases),
        Some("") => return Err("tab name is required".to_owned()),
        Some(name) if names::taken(&app.space.canvases, name, None) => {
            return Err(format!("a tab named '{name}' already exists"));
        }
        Some(name) => name.to_owned(),
    };
    let fresh = Parked {
        view: CanvasView {
            fitted: true,
            ..CanvasView::default()
        },
        ..Parked::default()
    };
    let canvas = new_canvas(&mut app.session, &app.space.canvases, name, fresh);
    let id = canvas.id.clone();
    app.space.canvases.push(canvas);
    effects.push(Effect::WriteCanvas(id.clone()));
    if activate_it {
        activate(app, app.space.canvases.len() - 1, effects);
    }
    effects.push(Effect::SaveSpaceMeta);
    Ok(id)
}

/// Gives the canvas `id` another name, and its file the name that goes
/// with it. Refused when the name is empty or another canvas has it.
pub(crate) fn rename(
    app: &mut App,
    id: &CanvasId,
    name: &str,
    effects: &mut Vec<Effect>,
) -> Result<(), String> {
    let name = name.trim();
    let Some(index) = app.space.index_of(id) else {
        return Err(format!("no canvas has the id '{id}'"));
    };
    if name.is_empty() {
        return Err("tab name is required".to_owned());
    }
    if app.space.canvases[index].name == name {
        return Ok(());
    }
    if names::taken(&app.space.canvases, name, Some(id)) {
        return Err(format!("a tab named '{name}' already exists"));
    }
    let to = canvas_file_name(name, id);
    let clash = (app.space.canvases.iter()).any(|canvas| canvas.id != *id && canvas.file == to);
    if clash {
        return Err(format!("another canvas is already kept in '{to}'"));
    }
    let canvas = &mut app.space.canvases[index];
    let from = mem::replace(&mut canvas.file, to.clone());
    name.clone_into(&mut canvas.name);
    if from != to {
        effects.push(Effect::RenameCanvasFile {
            canvas: id.clone(),
            from,
            to,
        });
    }
    effects.push(Effect::SaveSpaceMeta);
    Ok(())
}

/// Copies the canvas `id` into a new one just after it, named `<name>
/// Copy`, and shows the copy. Ids inside a canvas are its own, so the copy
/// keeps them. The copy starts with no undo history.
pub(crate) fn duplicate(
    app: &mut App,
    id: &CanvasId,
    effects: &mut Vec<Effect>,
) -> Option<CanvasId> {
    let index = app.space.index_of(id)?;
    let (document, camera) = app.canvas_to_save(id)?;
    let name = names::copy_of(&app.space.canvases, &app.space.canvases[index].name);
    let CanvasState::Parked(copy) = parked(document, camera, false) else {
        return None;
    };
    let canvas = new_canvas(&mut app.session, &app.space.canvases, name, *copy);
    let copy_id = canvas.id.clone();
    app.space.canvases.insert(index + 1, canvas);
    if index < app.space.active {
        app.space.active += 1;
    }
    effects.push(Effect::WriteCanvas(copy_id.clone()));
    activate(app, index + 1, effects);
    effects.push(Effect::SaveSpaceMeta);
    Some(copy_id)
}

/// Removes the canvas `id` and sends its file to the trash. Deleting the
/// active canvas shows the next one, or the one before the last. Deleting
/// the only canvas leaves an empty `Canvas 1` in its place, and returns
/// `Some(true)`.
pub(crate) fn delete(app: &mut App, id: &CanvasId, effects: &mut Vec<Effect>) -> Option<bool> {
    let index = app.space.index_of(id)?;
    let only = app.space.canvases.len() == 1;
    if only {
        let fresh = Parked {
            view: CanvasView {
                fitted: true,
                ..CanvasView::default()
            },
            ..Parked::default()
        };
        let name = DEFAULT_CANVAS_NAME.to_owned();
        let canvas = new_canvas(&mut app.session, &app.space.canvases, name, fresh);
        effects.push(Effect::WriteCanvas(canvas.id.clone()));
        app.space.canvases.push(canvas);
    }
    if index == app.space.active {
        let next = if index + 1 < app.space.canvases.len() {
            index + 1
        } else {
            index - 1
        };
        activate(app, next, effects);
    }
    let removed = app.space.canvases.remove(index);
    if index < app.space.active {
        app.space.active -= 1;
    }
    effects.push(Effect::TrashCanvasFile {
        canvas: removed.id,
        file: removed.file,
    });
    effects.push(Effect::SaveSpaceMeta);
    Some(only)
}

/// The file of the canvas `id` changed on disk and was read again. The
/// document replaces the one held and the canvas's undo history is
/// cleared: its steps were made against a document that is gone.
pub(crate) fn file_changed(
    app: &mut App,
    id: &CanvasId,
    document: Document,
    effects: &mut Vec<Effect>,
) {
    let Some(index) = app.space.index_of(id) else {
        return;
    };
    match &mut app.space.canvases[index].state {
        CanvasState::Active => open_document(app, document, effects),
        CanvasState::Parked(parked) => {
            let view = mem::take(&mut parked.view);
            **parked = Parked {
                document,
                history: specular_doc::History::default(),
                view: CanvasView {
                    camera: view.camera,
                    selection: view.selection,
                    ..CanvasView::default()
                },
            };
        }
    }
}

/// A parked canvas holding `document`.
fn parked(document: Document, camera: specular_core::Camera, fitted: bool) -> CanvasState {
    CanvasState::Parked(Box::new(Parked {
        document,
        history: specular_doc::History::default(),
        view: CanvasView {
            camera,
            fitted,
            ..CanvasView::default()
        },
    }))
}

/// A canvas named `name` with an id and a file name none of `others` has.
fn new_canvas(session: &mut Session, others: &[Canvas], name: String, state: Parked) -> Canvas {
    loop {
        let id = CanvasId::new(format!("tab_{}", session.next_id()));
        let file = canvas_file_name(&name, &id);
        let clash = (others.iter()).any(|canvas| canvas.id == id || canvas.file == file);
        if !clash {
            return Canvas {
                id,
                name,
                file,
                state: CanvasState::Parked(Box::new(state)),
            };
        }
    }
}

/// Makes the canvas at `index` the active one.
fn activate(app: &mut App, index: usize, effects: &mut Vec<Effect>) {
    let old = app.space.active;
    if index == old || index >= app.space.canvases.len() {
        return;
    }
    leave(app, effects);
    let before = pages::snapshot(&app.document);
    let leaving = Parked {
        document: mem::take(&mut app.document),
        history: mem::take(&mut app.history),
        view: CanvasView {
            camera: app.session.camera,
            selection: mem::take(&mut app.session.selection),
            loaded_fits: mem::take(&mut app.session.loaded_fits),
            fitted: true,
        },
    };
    let leaving = CanvasState::Parked(Box::new(leaving));
    let entering = mem::replace(&mut app.space.canvases[index].state, CanvasState::Active);
    app.space.canvases[old].state = leaving;
    app.space.active = index;
    app.space.switches += 1;
    if let CanvasState::Parked(entering) = entering {
        enter(app, *entering, &before, effects);
    }
}

/// Finishes what is in flight on the active canvas before it is parked. A
/// text edit ends and keeps its text; if that made an undo step, the
/// canvas is written, since it will not be the active one when the
/// autosave comes round.
fn leave(app: &mut App, effects: &mut Vec<Effect>) {
    let revision = app.history.revision();
    app.session.gesture = None;
    edit::end(app, effects);
    if app.history.is_open() {
        app.history.settle(app.session.selection.clone());
    }
    comment::forget(app);
    set_focus(app, None, effects);
    if app.history.revision() != revision {
        effects.push(Effect::WriteCanvas(app.space.active().id.clone()));
    }
}

/// Puts a parked canvas into the app's own fields, and brings the page
/// hosts, the images and the Documents in step with it. `before` is the
/// pages of the canvas that was showing.
fn enter(app: &mut App, parked: Parked, before: &[HostedPage], effects: &mut Vec<Effect>) {
    let Parked {
        document,
        history,
        view,
    } = parked;
    app.document = document;
    app.history = history;
    let session = &mut app.session;
    session.camera = view.camera;
    session.selection = view.selection;
    session.loaded_fits = view.loaded_fits;
    session.entered_group = None;
    session.hover = None;
    session.pointer_page = None;
    session.captured = crate::page_input::ButtonCapture::default();
    session.pages = crate::page_state::PageStates::default();
    if !view.fitted {
        edit::fit_all(app);
    }
    drop_dangling(app, effects);
    pages::replace(before, &app.document, effects);
    images::reopen(app, effects);
    notes::reopen(app, effects);
}
