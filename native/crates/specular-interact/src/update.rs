//! [`update`]: the one function that changes an [`App`].

use glam::DVec2;
use glam::Vec2;
use specular_core::InputEvent;
use specular_doc::{Command, CommandError, Document, EntityId, ItemId};

use crate::focus::{leave_unless_selected, set_focus};
use crate::images;
use crate::{
    Action, App, Effect, Event, Focus, PageNotice, camera, cursor, gesture, keys, pages, pointer,
    verbs,
};

/// Applies `event` to `app` and returns what the shell must now do, in
/// order. No I/O happens here.
pub fn update(app: &mut App, event: Event) -> Vec<Effect> {
    let mut effects = Vec::new();
    let revision = app.history.revision();
    // The clock moves nothing the cursor depends on.
    let ticks = matches!(event, Event::Tick { .. });
    match event {
        Event::Pointer(input) => pointer::on_pointer(app, &input, &mut effects),
        Event::Wheel(input) => camera::on_wheel(app, &input, &mut effects),
        Event::Pinch { delta } => camera::on_pinch(app, delta),
        Event::Key(input) => keys::on_key(app, &input, &mut effects),
        Event::Ime(ime) => match &app.session.focus {
            Focus::Page(page) => effects.push(Effect::ForwardInput {
                page: page.clone(),
                event: InputEvent::Ime(ime),
            }),
            Focus::Canvas => {}
        },
        Event::Page { page, notice } => on_page_notice(app, &page, &notice, &mut effects),
        Event::Image { image, notice } => images::on_notice(app, image, notice),
        Event::Tick { unix_ms } => app.session.now_ms = unix_ms,
        Event::ViewportResized(size) => app.session.viewport = size,
        Event::DocumentOpened(document) => open_document(app, *document, &mut effects),
        Event::Action(action) => run_action(app, action, &mut effects),
    }
    leave_unless_selected(app, &mut effects);
    // Every undoable change, undo and redo moves the history's revision, so
    // this is the one place a save is asked for, and the place to ask for
    // the images of file entities that just appeared.
    if app.history.revision() != revision {
        images::request_new(app, &mut effects);
        effects.push(Effect::Save);
    }
    if !ticks {
        cursor::refresh(app, &mut effects);
    }
    effects
}

pub(crate) fn run_action(app: &mut App, action: Action, effects: &mut Vec<Effect>) {
    match action {
        Action::Cancel => {
            // Escape is staged: it first backs out of whatever is in flight
            // (a drag, an armed tool, an entered page) and leaves the
            // selection alone. With nothing in flight it deselects.
            let session = &app.session;
            let idle = session.gesture.is_none()
                && session.tool == crate::Tool::Select
                && session.focus == Focus::Canvas;
            gesture::cancel(app);
            app.session.tool = crate::Tool::Select;
            set_focus(app, None, effects);
            if idle {
                app.session.selection.set([]);
            }
        }
        Action::SetTool(tool) => {
            if app.session.gesture.is_none() {
                app.session.tool = tool;
            }
        }
        Action::Undo => step_history(app, effects, |app| app.history.undo(&mut app.document)),
        Action::Redo => step_history(app, effects, |app| app.history.redo(&mut app.document)),
        Action::Select(items) => {
            app.session.selection.set(items);
            drop_dangling(app, effects);
        }
        Action::SetCamera(camera) => app.session.camera = camera,
        Action::Delete => verb(app, effects, verbs::delete),
        Action::Duplicate => verb(app, effects, verbs::duplicate),
        Action::Nudge { dx, dy } => verb(app, effects, |app, effects| {
            verbs::nudge(app, DVec2::new(dx, dy), effects);
        }),
    }
}

/// Runs a verb on the selection, unless a drag is in flight.
fn verb(app: &mut App, effects: &mut Vec<Effect>, run: impl FnOnce(&mut App, &mut Vec<Effect>)) {
    if app.session.gesture.is_none() {
        run(app, effects);
    }
}

/// Runs `command` as one undo step, then brings the page hosts and the
/// session back in step with the document.
pub(crate) fn document_step(app: &mut App, command: Command, effects: &mut Vec<Effect>) {
    let before = pages::snapshot(&app.document);
    gesture::apply_step(app, command);
    drop_dangling(app, effects);
    pages::reconcile(&before, &app.document, effects);
}

/// Runs an undo or a redo, unless a drag is in flight, and brings the page
/// hosts and the session back in step with the document.
fn step_history(
    app: &mut App,
    effects: &mut Vec<Effect>,
    step: impl FnOnce(&mut App) -> Result<bool, CommandError>,
) {
    if app.session.gesture.is_some() {
        return;
    }
    let before = pages::snapshot(&app.document);
    match step(app) {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => tracing::warn!("history step dropped: {error}"),
    }
    drop_dangling(app, effects);
    pages::reconcile(&before, &app.document, effects);
}

fn open_document(app: &mut App, document: Document, effects: &mut Vec<Effect>) {
    let before = pages::snapshot(&app.document);
    app.session.gesture = None;
    app.document = document;
    app.history.clear();
    drop_dangling(app, effects);
    pages::reconcile(&before, &app.document, effects);
    images::reopen(app, effects);
}

/// Forgets selection, hover and focus that name something the document no
/// longer holds.
fn drop_dangling(app: &mut App, effects: &mut Vec<Effect>) {
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
}

fn on_page_notice(app: &App, page: &EntityId, notice: &PageNotice, effects: &mut Vec<Effect>) {
    match notice {
        PageNotice::Loaded { .. } | PageNotice::Crashed { .. } => {}
        PageNotice::ImeCompositionBounds(bounds) => {
            if app.session.focus.page() == Some(page)
                && let Some(bounds) = bounds
                && let Some(placement) = app.page_placement(page)
            {
                let camera = &app.session.camera;
                let per_css = placement.canvas_per_css().as_vec2();
                let corner = Vec2::new(bounds.x as f32, bounds.y as f32);
                let world = placement.rect_origin() + corner * per_css;
                effects.push(Effect::SetImeCursorArea {
                    origin: camera.world_to_screen(world),
                    size: Vec2::new(bounds.width as f32, bounds.height as f32)
                        * per_css
                        * camera.zoom,
                });
            }
        }
    }
}
