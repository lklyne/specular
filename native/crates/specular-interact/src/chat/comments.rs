//! The comments a thread holds, as the document has them.

use specular_agent::{Thread, ThreadId};
use specular_doc::AnnotationId;

use crate::{App, Effect, comment};

/// The active canvas's id, which names its threads.
pub(super) fn tab(app: &App) -> String {
    app.space.active().id.as_str().to_owned()
}

/// The thread the panel shows, if one is open.
pub(super) fn active_thread(app: &App) -> Option<&Thread> {
    app.threads.active(app.space.active().id.as_str())
}

/// The open thread's id.
pub(super) fn active_id(app: &App) -> Option<ThreadId> {
    active_thread(app).map(|thread| thread.id.clone())
}

/// The comments of `thread` that were sent, are still in the document and
/// are still open: what "Resolve comments" closes. A comment still waiting
/// in the queue is not one.
pub(super) fn open_sent(app: &App, thread: &Thread) -> Vec<AnnotationId> {
    let queued = app.threads.queued(&thread.id);
    thread
        .annotation_ids
        .iter()
        .filter(|id| {
            !queued
                .iter()
                .any(|m| m.annotation_id.as_deref() == Some(id))
        })
        .map(|id| AnnotationId::new(id.as_str()))
        .filter(|id| {
            app.document
                .annotation(id)
                .is_some_and(|annotation| comment::is_open(annotation.status))
        })
        .collect()
}

/// Resolves the open thread's open, sent comments as one undo step.
pub(super) fn resolve_sent(app: &mut App, effects: &mut Vec<Effect>) {
    let Some(thread) = active_thread(app) else {
        return;
    };
    let ids = open_sent(app, thread);
    comment::resolve_all(app, &ids, effects);
}
