//! What can be done to the canvases of the space: open a space, switch,
//! new, rename, duplicate and delete. Each changes the list in memory and
//! returns the effects that put the same change on disk.
//!
//! A background canvas has no page hosts. Switching closes every page of
//! the canvas being left and creates every page of the one being entered,
//! so a canvas costs nothing while it is not shown and a page reloads when
//! its canvas comes back.

mod projects;
mod swap;

pub(crate) use self::projects::{add_project, project_act, remove_project};

use self::swap::{activate, enter, new_canvas, parked};

use std::mem;

use specular_doc::Document;

use super::{
    Canvas, CanvasId, CanvasState, CanvasView, DEFAULT_CANVAS_NAME, OpenedCanvas, OpenedSpace,
    Parked, ProjectId, SavedCanvas, Space, canvas_file_name, names,
};
use crate::focus::set_focus;
use crate::pages::{self};
use crate::showing::Tabs;
use crate::update::open_document;
use crate::{App, CanvasAction, Effect, comment, edit};

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
        CanvasAction::New(project) => {
            let project = project.unwrap_or_else(|| app.space.active().project.clone());
            create(app, &project, None, true, effects).map(|_| ())
        }
        CanvasAction::Rename { canvas, name } => {
            let canvas = target(app, canvas);
            rename(app, &canvas, &name, effects)
        }
        CanvasAction::BeginRename(canvas) => {
            let canvas = target(app, canvas);
            crate::sidebar::begin_rename(app, &canvas, effects);
            Ok(())
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
    let before = pages::snapshot(app);
    app.space_ask = None;
    app.session.gesture = None;
    edit::discard(app, effects);
    comment::forget(app);
    set_focus(app, None, effects);
    crate::showing::restore(app);
    app.session.tabs = Tabs::default();

    let OpenedSpace {
        project,
        canvases,
        active,
    } = opened;
    let project_id = project
        .as_ref()
        .map_or_default(|project| project.id.clone());
    let mut entries: Vec<Canvas> = Vec::new();
    for canvas in canvases {
        let entry = entry(&entries, &project_id, canvas);
        entries.push(entry);
    }
    let made = entries.is_empty().then(|| {
        let first = new_canvas(
            &mut app.session,
            &entries,
            &project_id,
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
    // The threads belong to the folder: they are read again from it.
    let has_folder = project.is_some();
    app.threads = specular_agent::Threads::default();
    app.space = Space {
        projects: project.into_iter().collect(),
        canvases: entries,
        active: index,
        switches: app.space.switches + 1,
    };
    app.history = crate::shared::Undo::default();
    if let CanvasState::Parked(entering) = entering {
        enter(app, *entering, &before, effects);
    }
    if let Some(id) = made {
        effects.push(Effect::WriteCanvas(id));
        effects.push(Effect::SaveSpaceMeta);
    }
    if has_folder {
        effects.push(Effect::LoadThreads);
    }
}

/// A canvas read from disk, among `others`, in the project `project`. Its
/// index's id is kept unless another canvas has it; then it is known by
/// the id with the project's after it, and its index still names it as
/// before.
pub(super) fn entry(others: &[Canvas], project: &ProjectId, canvas: OpenedCanvas) -> Canvas {
    let tab = canvas.id.as_str().to_owned();
    let taken = |id: &CanvasId| others.iter().any(|other| other.id == *id);
    let id = if taken(&canvas.id) {
        CanvasId::new(format!(
            "{tab}~{}",
            project.as_str().get(..6).unwrap_or(project.as_str())
        ))
    } else {
        canvas.id
    };
    Canvas {
        id,
        project: project.clone(),
        tab,
        name: canvas.name,
        file: canvas.file,
        state: parked(canvas.document, canvas.camera, canvas.tabs),
    }
}

/// An empty canvas that needs no fitting.
pub(super) fn fresh() -> Parked {
    Parked {
        view: CanvasView {
            fitted: true,
            ..CanvasView::default()
        },
        ..Parked::default()
    }
}

/// Where a new canvas of the project `project` goes in the list: after its
/// last canvas, or at the end.
fn slot_for(space: &Space, project: &ProjectId) -> usize {
    (space.canvases.iter())
        .rposition(|canvas| canvas.project == *project)
        .map_or(space.canvases.len(), |last| last + 1)
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

/// Adds an empty canvas after the last one of the project `project`. With
/// no `name` it is the next `Canvas N`. A name is refused when it is empty
/// or taken in that project: a tab ref resolves by exact name, so a second
/// canvas with one would make every ref to it ambiguous.
pub(crate) fn create(
    app: &mut App,
    project: &ProjectId,
    name: Option<&str>,
    activate_it: bool,
    effects: &mut Vec<Effect>,
) -> Result<CanvasId, String> {
    if !app.space.projects.is_empty() && app.space.project(project).is_none() {
        return Err(format!("no project has the id '{project}'"));
    }
    let siblings = app.space.siblings(project);
    let name = match name.map(str::trim) {
        None => names::next_default(&siblings),
        Some("") => return Err("tab name is required".to_owned()),
        Some(name) if names::taken(&siblings, name, None) => {
            return Err(format!("a tab named '{name}' already exists"));
        }
        Some(name) => name.to_owned(),
    };
    let canvas = new_canvas(
        &mut app.session,
        &app.space.canvases,
        project,
        name,
        fresh(),
    );
    let id = canvas.id.clone();
    let slot = slot_for(&app.space, project);
    app.space.canvases.insert(slot, canvas);
    if slot <= app.space.active {
        app.space.active += 1;
    }
    effects.push(Effect::WriteCanvas(id.clone()));
    if activate_it {
        activate(app, slot, effects);
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
    let project = app.space.canvases[index].project.clone();
    if names::taken(&app.space.siblings(&project), name, Some(id)) {
        return Err(format!("a tab named '{name}' already exists"));
    }
    let to = canvas_file_name(name, &CanvasId::new(app.space.canvases[index].tab.clone()));
    let clash =
        (app.space.canvases_of(&project)).any(|canvas| canvas.id != *id && canvas.file == to);
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
    let SavedCanvas {
        document, camera, ..
    } = app.canvas_to_save(id)?;
    let project = app.space.canvases[index].project.clone();
    let name = names::copy_of(
        &app.space.siblings(&project),
        &app.space.canvases[index].name,
    );
    let CanvasState::Parked(copy) = parked(document, camera, Vec::new()) else {
        return None;
    };
    let canvas = new_canvas(&mut app.session, &app.space.canvases, &project, name, *copy);
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
/// the only canvas of its project leaves an empty `Canvas 1` there in its
/// place, and returns `Some(true)`.
pub(crate) fn delete(app: &mut App, id: &CanvasId, effects: &mut Vec<Effect>) -> Option<bool> {
    let index = app.space.index_of(id)?;
    let project = app.space.canvases[index].project.clone();
    let only = app.space.canvases_of(&project).count() == 1;
    if only {
        let name = DEFAULT_CANVAS_NAME.to_owned();
        let canvas = new_canvas(
            &mut app.session,
            &app.space.canvases,
            &project,
            name,
            fresh(),
        );
        effects.push(Effect::WriteCanvas(canvas.id.clone()));
        app.space.canvases.insert(index + 1, canvas);
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
            // A shared canvas stays shared; its peers get the new file's
            // contents on the next flush.
            let mut history = mem::take(&mut parked.history);
            history.clear();
            **parked = Parked {
                document,
                history,
                view: CanvasView {
                    camera: view.camera,
                    selection: view.selection,
                    tabs: view.tabs,
                    ..CanvasView::default()
                },
            };
        }
    }
}
