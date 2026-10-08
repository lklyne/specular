//! What a run reported, and what the disk held.

use serde_json::Value;
use specular_agent::{Index, Notice, Outcome, Thread, ThreadId};
use specular_doc::AnnotationId;

use super::action::now;
use super::{comments, effects, send};
use crate::{App, Effect};

/// The threads read from `.specular/threads/` replace the store.
pub(crate) fn on_loaded(app: &mut App, threads: Vec<Thread>, index: &Index) {
    app.threads.load(threads, index);
}

/// One thing the run of `thread` said. A finished run whose thread has more
/// queued is run again at once, aimed at the last queued pin; a failed
/// resume starts over without the session.
pub(crate) fn on_agent(
    app: &mut App,
    thread: &ThreadId,
    notice: Notice,
    effects: &mut Vec<Effect>,
) {
    let message = format!("tmsg_{}", app.fresh_id());
    let now = now(app);
    match app.threads.on_notice(thread, notice, &message, &now) {
        Outcome::Ignored | Outcome::Progressed | Outcome::Failed | Outcome::Cancelled => {}
        Outcome::Finished {
            has_queued_turn,
            changed,
        } => {
            let started = has_queued_turn
                .then(|| {
                    let aim = app
                        .threads
                        .last_queued_annotation(thread)
                        .map(str::to_owned);
                    send::begin(app, thread, aim.as_deref())
                })
                .flatten();
            effects::run_started(started, changed, effects);
        }
        Outcome::Retry { request, changed } => {
            effects::emit(&changed, effects);
            effects.push(Effect::RunAgent(Box::new(request)));
        }
    }
}

/// Opens the thread the comment `id` was queued into, when the annotation
/// names one that is still live on the active canvas.
pub(crate) fn open_thread_of(app: &mut App, id: &AnnotationId, effects: &mut Vec<Effect>) {
    let named = (app.document.annotation(id))
        .and_then(|annotation| annotation.metadata.as_ref())
        .and_then(|metadata| metadata.get("threadId"))
        .and_then(Value::as_str)
        .map(|thread| ThreadId(thread.to_owned()));
    if let Some(thread) = named {
        let changed = app.threads.select(&comments::tab(app), &thread);
        effects::emit(&changed, effects);
    }
}
