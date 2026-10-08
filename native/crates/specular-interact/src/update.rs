//! [`update`]: the one function that changes an [`App`].

use glam::DVec2;
use glam::Vec2;
use specular_core::InputEvent;
use specular_doc::{Command, CommandError, Document, EntityId, ItemId};

use crate::focus::{leave_unless_selected, set_focus};
use crate::images;
use crate::notes;
use crate::stack_order::Move;
use crate::{
    Action, App, Effect, Event, Focus, PageNotice, Selection, ToolDefaultPatch, api, bindings,
    camera, comment, cursor, edit, gesture, groups, page_state, pages, pointer, space, verbs,
};
use crate::{clipboard, drop, select_all, zoom};

/// Applies `event` to `app` and returns what the shell must now do, in
/// order. No I/O happens here.
pub fn update(app: &mut App, event: Event) -> Vec<Effect> {
    let mut effects = Vec::new();
    let revision = app.history.revision();
    let switches = app.space.switches();
    let camera = app.session.camera;
    let dragging = app.session.gesture.is_some();
    // The clock moves nothing the cursor depends on.
    let ticks = matches!(event, Event::Tick { .. });
    let caret = edit::caret_state(app);
    // A pointer event has already put the drag and the hover where it is,
    // unless it ended the drag: the hover is not kept up during one.
    let pointing = matches!(event, Event::Pointer(_));
    match event {
        Event::Pointer(input) => pointer::on_pointer(app, &input, &mut effects),
        Event::Wheel(input) => {
            if !notes::on_wheel(app, &input) {
                camera::on_wheel(app, &input, &mut effects);
            }
        }
        Event::Pinch { delta } => camera::on_pinch(app, delta),
        Event::Key(input) => bindings::on_key(app, &input, &mut effects),
        Event::Ime(ime) => match &app.session.focus {
            Focus::Page(page) => effects.push(Effect::ForwardInput {
                page: page.clone(),
                event: InputEvent::Ime(ime),
            }),
            Focus::Canvas => edit::on_ime(app, &ime, &mut effects),
        },
        Event::Page { page, notice } => {
            // The address is the one thing a page reports that is saved.
            if page_state::on_notice(app, &page, &notice) {
                effects.push(Effect::Save);
            }
            on_page_notice(app, &page, &notice, &mut effects);
        }
        Event::Image { image, notice } => images::on_notice(app, image, notice),
        Event::Note { file, notice } => notes::on_notice(app, &file, notice, &mut effects),
        Event::NoteCreated { file, rect } => edit::note::created(app, file, rect, &mut effects),
        Event::NoteHeights(heights) => notes::on_heights(app, heights),
        Event::Tick { unix_ms } => {
            let elapsed = unix_ms.saturating_sub(app.session.now_ms);
            app.session.now_ms = unix_ms;
            edit::note::autosave(app, &mut effects);
            edit::autoscroll(app, elapsed);
        }
        Event::ViewportResized(size) => app.session.viewport = size,
        Event::DocumentOpened(document) => open_document(app, *document, &mut effects),
        Event::SpaceOpened(opened) => space::open(app, *opened, &mut effects),
        Event::CanvasFileChanged { canvas, document } => {
            space::file_changed(app, &canvas, *document, &mut effects);
        }
        Event::Clipboard(content) => clipboard::on_read(app, content, &mut effects),
        Event::FilesDropped { files, screen } => drop::on_drop(app, &files, screen, &mut effects),
        Event::ElementAt {
            page,
            point,
            element,
        } => comment::on_element(app, &page, point, element, &mut effects),
        Event::RegionGrab { region, grabs } => {
            comment::on_region_grab(app, region, &grabs, &mut effects);
        }
        Event::ToolDefaultsLoaded(defaults) => app.tool_defaults = *defaults,
        Event::Action(action) => run_action(app, action, &mut effects),
        Event::Api(call) => api::run(app, call, &mut effects),
    }
    // Another canvas has another history: its revision says nothing about
    // whether this event made a step.
    let switched = app.space.switches() != switches;
    let stepped = !switched && app.history.revision() != revision;
    let moved = switched || stepped || app.session.camera != camera;
    let drag_ended = dragging && app.session.gesture.is_none();
    if (moved && !pointing) || drag_ended {
        pointer::settle(app);
    }
    // Whatever made a step has selected what it meant to by now.
    if app.history.is_open() {
        app.history.settle(app.session.selection.clone());
    }
    leave_unless_selected(app, &mut effects);
    comment::settle(app);
    groups::keep_entered_valid(app);
    if edit::restart_blink(app, &caret) {
        notes::reveal_caret(app);
    }
    // Every undoable change, undo and redo moves the history's revision, so
    // this is the one place a save is asked for, and the place to ask for
    // the images and markdown files of file entities that just appeared.
    if stepped {
        images::request_new(app, &mut effects);
        notes::request_new(app, &mut effects);
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
            // A comment draft or a focused comment is all one Escape takes.
            if comment::cancel(app, effects) {
                return;
            }
            // Escape is staged: it first backs out of whatever is in flight
            // (a drag, an armed tool, a text edit, an entered page) and
            // leaves the selection alone. Then it steps out of an entered
            // group, selecting it. With nothing to back out of it
            // deselects. A text edit is kept, not thrown away.
            let session = &app.session;
            let idle = session.gesture.is_none()
                && session.tool == crate::Tool::Select
                && session.editing.is_none()
                && session.focus == Focus::Canvas;
            gesture::cancel(app, effects);
            app.session.tool = crate::Tool::Select;
            let stepped_out = idle && groups::step_out(app);
            // A title edit is abandoned by Escape; other text keeps what
            // was typed.
            if edit::is_editing_title(app) {
                edit::discard(app, effects);
            }
            edit::end(app, effects);
            set_focus(app, None, effects);
            if idle && !stepped_out {
                app.session.selection.set([]);
            }
        }
        Action::SetTool(tool) => {
            if app.session.gesture.is_none() {
                edit::end(app, effects);
                app.session.tool = tool;
            }
        }
        Action::SetToolDefault(patch) => set_tool_default(app, patch, effects),
        Action::SetToolVariant(patch) => {
            if app.session.gesture.is_none() {
                edit::end(app, effects);
                app.session.tool = patch.tool();
                set_tool_default(app, patch, effects);
            }
        }
        // While text is edited, undo and redo are the editor's own.
        Action::Undo | Action::Redo if app.session.editing.is_some() => {
            if app.session.gesture.is_none() {
                edit::step_history(app, action == Action::Undo);
            }
        }
        Action::Undo => step_history(app, effects, |app| app.history.undo(&mut app.document)),
        Action::Redo => step_history(app, effects, |app| app.history.redo(&mut app.document)),
        Action::Select(items) => {
            app.session.selection.set(items);
            drop_dangling(app, effects);
        }
        Action::SetCamera(camera) => app.session.camera = camera,
        // The focus and the selection are never both set, so Delete has one
        // thing to remove.
        Action::Delete if app.session.focused_comment.is_some() => {
            verb(app, effects, |app, effects| {
                comment::delete(app, None, effects);
            });
        }
        Action::Delete => verb(app, effects, verbs::delete),
        Action::Duplicate => verb(app, effects, verbs::duplicate),
        Action::Nudge { dx, dy } => verb(app, effects, |app, effects| {
            verbs::nudge(app, DVec2::new(dx, dy), effects);
        }),
        Action::Format(format) => {
            if app.session.gesture.is_none() {
                edit::format(app, format);
            }
        }
        Action::BringForward => stack(app, Move::Forward, effects),
        Action::SendBackward => stack(app, Move::Backward, effects),
        Action::BringToFront => stack(app, Move::ToFront, effects),
        Action::SendToBack => stack(app, Move::ToBack, effects),
        Action::AnnotateSelection => verb(app, effects, comment::annotate_selection),
        Action::FocusComment(id) => verb(app, effects, |app, _| comment::focus(app, id.as_ref())),
        Action::ResolveComment(id) => verb(app, effects, |app, effects| {
            comment::resolve(app, id.as_ref(), effects);
        }),
        Action::DeleteComment(id) => verb(app, effects, |app, effects| {
            comment::delete(app, id.as_ref(), effects);
        }),
        Action::Group => verb(app, effects, groups::group),
        Action::Ungroup => verb(app, effects, groups::ungroup),
        Action::Copy => clipboard::copy(app, effects),
        Action::Cut => verb(app, effects, clipboard::cut),
        Action::Paste => clipboard::request(app, effects),
        Action::SelectAll => verb(app, effects, |app, _| select_all::run(app)),
        Action::ZoomIn => zoom::zoom_in(app),
        Action::ZoomOut => zoom::zoom_out(app),
        Action::ZoomReset => zoom::reset(app),
        Action::ZoomToFit => zoom::to_fit(app),
        Action::PageBack | Action::PageForward | Action::PageReload | Action::PageStop => {
            page_state::navigate(app, &action, effects);
        }
        Action::Canvas(action) => space::act(app, action, effects),
    }
}

fn stack(app: &mut App, how: Move, effects: &mut Vec<Effect>) {
    verb(app, effects, |app, effects| {
        verbs::reorder(app, how, effects);
    });
}

/// Changes one tool default, and asks for the defaults to be saved if that
/// changed anything.
fn set_tool_default(app: &mut App, patch: ToolDefaultPatch, effects: &mut Vec<Effect>) {
    let before = app.tool_defaults.clone();
    app.tool_defaults.apply(patch);
    if app.tool_defaults != before {
        effects.push(Effect::SaveToolDefaults(Box::new(
            app.tool_defaults.clone(),
        )));
    }
}

/// Runs a verb on the selection, unless a drag is in flight. A text edit
/// ends first, so the verb acts on its result.
fn verb(app: &mut App, effects: &mut Vec<Effect>, run: impl FnOnce(&mut App, &mut Vec<Effect>)) {
    if app.session.gesture.is_none() {
        edit::end(app, effects);
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

/// Runs an undo or a redo, unless a drag is in flight, puts back the
/// selection the step carries, and brings the page hosts and the session
/// back in step with the document.
fn step_history(
    app: &mut App,
    effects: &mut Vec<Effect>,
    step: impl FnOnce(&mut App) -> Result<Option<Selection>, CommandError>,
) {
    if app.session.gesture.is_some() {
        return;
    }
    let before = pages::snapshot(&app.document);
    let notes_before = edit::note::held(app);
    match step(app) {
        Ok(Some(selection)) => app.session.selection = selection,
        Ok(None) => return,
        Err(error) => tracing::warn!("history step dropped: {error}"),
    }
    edit::note::write_stepped(app, &notes_before, effects);
    drop_dangling(app, effects);
    pages::reconcile(&before, &app.document, effects);
}

pub(crate) fn open_document(app: &mut App, document: Document, effects: &mut Vec<Effect>) {
    let before = pages::snapshot(&app.document);
    app.session.gesture = None;
    app.session.entered_group = None;
    edit::discard(app, effects);
    comment::forget(app);
    app.document = document;
    edit::fit_all(app);
    app.history.clear();
    drop_dangling(app, effects);
    pages::reconcile(&before, &app.document, effects);
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
}

fn on_page_notice(app: &App, page: &EntityId, notice: &PageNotice, effects: &mut Vec<Effect>) {
    match notice {
        PageNotice::Loaded { .. }
        | PageNotice::Crashed { .. }
        | PageNotice::Title(_)
        | PageNotice::Url(_)
        | PageNotice::Loading { .. }
        | PageNotice::Scrolled { .. }
        | PageNotice::DevtoolsUrl(_) => {}
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
