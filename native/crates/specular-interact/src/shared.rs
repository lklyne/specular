//! Canvases shared with peers (ADR 0048). A shared canvas's undo steps go
//! through a [`CanvasSync`] instead of the plain [`History`], whatever
//! changes its document reaches peers after each event, and a peer's update
//! lands as commands with no undo step.
//!
//! Carrying updates between peers is the shell's: it hears
//! [`Effect::ShareSend`] and answers with [`ShareEvent::Receive`].

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use specular_doc::{Command, CommandError, Document, History};
use specular_sync::CanvasSync;

use crate::space::{CanvasId, Parked};
use crate::update::{drop_dangling, open_document};
use crate::{ApiOutcome, App, Effect, Selection, edit, pages};

/// A shared canvas's part in an [`Event`](crate::Event).
#[derive(Debug, Clone, PartialEq)]
pub enum ShareEvent {
    /// Share the active canvas. Its whole state goes out as one
    /// [`Effect::ShareSend`], for the transport to publish.
    Start,
    /// Show a peer's shared canvas in the active one: `state` is the whole
    /// shared document, as a `Start` sent it. The canvas's document is
    /// replaced.
    Join(Vec<u8>),
    /// A peer's update for a shared canvas, the active one or another.
    Receive {
        /// The canvas.
        canvas: CanvasId,
        /// The yrs update.
        update: Vec<u8>,
    },
    /// The live room of a canvas [`Effect::ShareConnect`] connected
    /// answered with `state`, everything shared so far. Empty, the room is
    /// new and the canvas is shared from here; otherwise the canvas shows
    /// the shared one. Again after a dropped connection, the two are
    /// merged and the canvas sends what the room may have missed.
    Connected {
        /// The canvas.
        canvas: CanvasId,
        /// The room's whole state, as one yrs update.
        state: Vec<u8>,
    },
    /// Stop sharing the active canvas. Its document stays as it is.
    Stop,
}

/// A canvas's undo steps: a plain [`History`], or the shared document's
/// while the canvas is shared.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Undo {
    local: History<Selection>,
    shared: Option<Shared>,
}

/// The shared document of one canvas. A clone of an [`App`] shares it with
/// the original, as two views of one canvas would.
#[derive(Clone)]
struct Shared(Arc<Mutex<CanvasSync<Selection>>>);

impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.lock().fmt(f)
    }
}

impl PartialEq for Shared {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Shared {
    fn new(sync: CanvasSync<Selection>) -> Self {
        Self(Arc::new(Mutex::new(sync)))
    }

    fn lock(&self) -> MutexGuard<'_, CanvasSync<Selection>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Undo {
    /// Whether the canvas is shared.
    pub(crate) fn is_shared(&self) -> bool {
        self.shared.is_some()
    }

    /// Runs `command` on `document` as one undo step.
    pub(crate) fn apply_from(
        &mut self,
        document: &mut Document,
        command: Command,
        before: Selection,
    ) -> Result<(), CommandError> {
        match &self.shared {
            Some(shared) => shared.lock().apply_from(document, command, before),
            None => self.local.apply_from(document, command, before),
        }
    }

    pub(crate) fn settle(&mut self, after: Selection) {
        match &self.shared {
            Some(shared) => shared.lock().settle(after),
            None => self.local.settle(after),
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        match &self.shared {
            Some(shared) => shared.lock().is_open(),
            None => self.local.is_open(),
        }
    }

    pub(crate) fn undo(
        &mut self,
        document: &mut Document,
    ) -> Result<Option<Selection>, CommandError> {
        match &self.shared {
            Some(shared) => Ok(shared.lock().undo(document)),
            None => self.local.undo(document),
        }
    }

    pub(crate) fn redo(
        &mut self,
        document: &mut Document,
    ) -> Result<Option<Selection>, CommandError> {
        match &self.shared {
            Some(shared) => Ok(shared.lock().redo(document)),
            None => self.local.redo(document),
        }
    }

    pub(crate) fn can_undo(&self) -> bool {
        match &self.shared {
            Some(shared) => shared.lock().can_undo(),
            None => self.local.can_undo(),
        }
    }

    pub(crate) fn can_redo(&self) -> bool {
        match &self.shared {
            Some(shared) => shared.lock().can_redo(),
            None => self.local.can_redo(),
        }
    }

    /// Goes up with every step, undo, redo and applied peer change. Both
    /// counts only ever grow, so their sum does too across a start or stop.
    pub(crate) fn revision(&self) -> u64 {
        let shared = self
            .shared
            .as_ref()
            .map_or(0, |shared| shared.lock().revision());
        self.local.revision() + shared
    }

    /// Forgets every step, as after the document was replaced. A shared
    /// canvas stays shared.
    pub(crate) fn clear(&mut self) {
        self.local.clear();
        if let Some(shared) = &self.shared {
            shared.lock().clear_undo();
        }
    }

    /// Shares whatever `document` holds that peers have not seen, and
    /// returns the update to send them.
    fn flush(&self, document: &Document) -> Option<Vec<u8>> {
        let shared = self.shared.as_ref()?;
        let mut sync = shared.lock();
        sync.reconcile(document);
        sync.take_outgoing()
    }
}

pub(crate) fn on_event(app: &mut App, event: ShareEvent, effects: &mut Vec<Effect>) {
    match event {
        ShareEvent::Start => start(app, effects),
        ShareEvent::Join(state) => join(app, &state, effects),
        ShareEvent::Receive { canvas, update } => receive(app, &canvas, &update, effects),
        ShareEvent::Connected { canvas, state } => connected(app, &canvas, &state, effects),
        ShareEvent::Stop => {
            if app.history.shared.take().is_some() {
                effects.push(Effect::ShareDisconnect(app.space.active().id.clone()));
            }
        }
    }
}

/// Starts connecting the active canvas, or a new one with `fresh`, to the
/// live room `link` opens.
pub(crate) fn connect(
    app: &mut App,
    link: String,
    fresh: bool,
    effects: &mut Vec<Effect>,
) -> ApiOutcome {
    if fresh {
        if let Err(reason) = crate::space::create(app, None, true, effects) {
            return ApiOutcome::Refused(reason);
        }
    } else if app.history.is_shared() {
        return ApiOutcome::Refused("this canvas is shared already".to_owned());
    }
    let canvas = app.space.active().id.clone();
    effects.push(Effect::ShareConnect { canvas, link });
    ApiOutcome::Done
}

fn connected(app: &mut App, canvas: &CanvasId, state: &[u8], effects: &mut Vec<Effect>) {
    let shared = if *canvas == app.space.active().id {
        app.history.shared.clone()
    } else {
        let parked = app.space.parked_mut(canvas);
        parked.and_then(|parked| parked.history.shared.clone())
    };
    if let Some(shared) = shared {
        // Back after a drop: take what the room has, and send it everything
        // this canvas holds, which may include edits it never got.
        receive(app, canvas, state, effects);
        let update = shared.lock().encode_state();
        effects.push(Effect::ShareSend {
            canvas: canvas.clone(),
            update,
        });
        return;
    }
    if *canvas != app.space.active().id {
        tracing::warn!("a share connected after its canvas was left; it is closed");
        effects.push(Effect::ShareDisconnect(canvas.clone()));
        return;
    }
    if state.is_empty() {
        start(app, effects);
    } else {
        join(app, state, effects);
    }
}

/// The steps made before sharing were made against no shared document, so
/// they cannot be undone through one and are forgotten.
fn start(app: &mut App, effects: &mut Vec<Effect>) {
    if app.history.is_shared() {
        return;
    }
    let sync = CanvasSync::new(&app.document);
    let state = sync.encode_state();
    app.history.local.clear();
    app.history.shared = Some(Shared::new(sync));
    effects.push(Effect::ShareSend {
        canvas: app.space.active().id.clone(),
        update: state,
    });
}

fn join(app: &mut App, state: &[u8], effects: &mut Vec<Effect>) {
    match CanvasSync::join(state) {
        Ok((sync, document)) => {
            open_document(app, document, effects);
            app.history.shared = Some(Shared::new(sync));
            effects.push(Effect::Save);
        }
        Err(error) => tracing::warn!("shared canvas not joined: {error}"),
    }
}

fn receive(app: &mut App, canvas: &CanvasId, update: &[u8], effects: &mut Vec<Effect>) {
    if *canvas != app.space.active().id {
        let found = app.space.parked_mut(canvas).is_some_and(|parked| {
            let Parked {
                document, history, ..
            } = parked;
            history.receive(update, document)
        });
        if found {
            effects.push(Effect::WriteCanvas(canvas.clone()));
        }
        return;
    }
    // As an undo does, keep the page hosts, the notes and the selection in
    // step with what the peer changed. Unlike an undo it lands mid-drag
    // too: the drag's next frame writes over any field they share.
    let before = pages::snapshot(app);
    let notes_before = edit::note::held(app);
    if !app.history.receive(update, &mut app.document) {
        return;
    }
    edit::note::write_stepped(app, &notes_before, effects);
    drop_dangling(app, effects);
    pages::reconcile(&before, app, effects);
}

impl App {
    /// Whether the canvas the window shows is shared.
    pub fn is_shared(&self) -> bool {
        self.history.is_shared()
    }
}

impl Undo {
    /// Applies a peer's update to `document`. Whether it changed anything:
    /// not when the canvas is not shared or the update was refused.
    fn receive(&mut self, update: &[u8], document: &mut Document) -> bool {
        let Some(shared) = &self.shared else {
            return false;
        };
        match shared.lock().apply_remote(update, document) {
            Ok(ran) => ran.is_some(),
            Err(error) => {
                tracing::warn!("a peer's update was refused: {error}");
                false
            }
        }
    }
}

/// After every event: whatever each shared canvas's document holds that
/// its peers have not seen goes out.
pub(crate) fn flush(app: &mut App, effects: &mut Vec<Effect>) {
    if let Some(update) = app.history.flush(&app.document) {
        effects.push(Effect::ShareSend {
            canvas: app.space.active().id.clone(),
            update,
        });
    }
    for (canvas, parked) in app.space.parked() {
        if let Some(update) = parked.history.flush(&parked.document) {
            effects.push(Effect::ShareSend {
                canvas: canvas.clone(),
                update,
            });
        }
    }
}
