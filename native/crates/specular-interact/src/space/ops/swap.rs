//! Parking the canvas that was showing and bringing another in.

use std::mem;

use specular_doc::Document;

use super::super::{Canvas, CanvasId, CanvasState, CanvasView, Parked, canvas_file_name};
use crate::focus::set_focus;
use crate::pages::{self, HostedPage};
use crate::showing::{SavedTab, Tabs};
use crate::update::drop_dangling;
use crate::{App, Effect, Session, comment, edit, images, notes};

/// A parked canvas holding `document` as it was read, with the item tabs
/// it was saved with.
pub(super) fn parked(
    document: Document,
    camera: specular_core::Camera,
    tabs: Vec<SavedTab>,
) -> CanvasState {
    let tabs = Tabs::from_saved(tabs, &document);
    CanvasState::Parked(Box::new(Parked {
        document,
        history: specular_doc::History::default(),
        view: CanvasView {
            camera,
            tabs,
            ..CanvasView::default()
        },
    }))
}

/// A canvas named `name` with an id and a file name none of `others` has.
pub(super) fn new_canvas(
    session: &mut Session,
    others: &[Canvas],
    name: String,
    state: Parked,
) -> Canvas {
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
pub(super) fn activate(app: &mut App, index: usize, effects: &mut Vec<Effect>) {
    let old = app.space.active;
    if index == old || index >= app.space.canvases.len() {
        return;
    }
    leave(app, effects);
    let before = pages::snapshot(app);
    let leaving = Parked {
        document: mem::take(&mut app.document),
        history: mem::take(&mut app.history),
        view: CanvasView {
            camera: app.session.camera,
            selection: mem::take(&mut app.session.selection),
            tabs: mem::take(&mut app.session.tabs),
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
    crate::showing::restore(app);
    if app.history.revision() != revision {
        effects.push(Effect::WriteCanvas(app.space.active().id.clone()));
    }
}

/// Puts a parked canvas into the app's own fields, and brings the page
/// hosts, the images and the Documents in step with it. `before` is the
/// pages of the canvas that was showing.
pub(super) fn enter(
    app: &mut App,
    parked: Parked,
    before: &[HostedPage],
    effects: &mut Vec<Effect>,
) {
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
    session.tabs = view.tabs;
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
    pages::replace(before, app, effects);
    images::reopen(app, effects);
    notes::reopen(app, effects);
}
