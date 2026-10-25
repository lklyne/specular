//! Sending comments to the agent to fix: what `POST /annotations/fix` asks,
//! one comment or every open one on an origin (Electron's
//! `fixAnnotation` and `fixPendingAnnotationsForOrigin`).
//!
//! A comment is fixed only on an origin bound to a repo, so the agent runs
//! where the code is. A comment already in a thread is sent on that thread;
//! one with none (made over the API) joins the canvas's draft first. A
//! thread whose agent is working is left alone: what it holds is sent when
//! the run ends.

use specular_agent::ThreadId;
use specular_doc::AnnotationId;

use super::action::now;
use super::{comments, effects, prompt, send};
use crate::{App, Effect, comment};

/// What to fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixTarget {
    /// One comment.
    Comment(AnnotationId),
    /// Every open comment on pages of this origin.
    Origin(String),
}

/// The comments `target` sends to the agent now, in document order.
pub fn fixable(app: &App, target: &FixTarget) -> Vec<AnnotationId> {
    (app.document.annotations().iter())
        .filter(|annotation| match target {
            FixTarget::Comment(id) => annotation.id == *id,
            FixTarget::Origin(origin) => {
                prompt::comment_origin(app, annotation).as_deref() == Some(origin.as_str())
            }
        })
        .filter(|annotation| comment::is_open(annotation.status))
        .filter(|annotation| {
            prompt::comment_origin(app, annotation)
                .is_some_and(|origin| app.repos.binding(&origin).is_some())
        })
        .filter(|annotation| {
            (app.threads.thread_of_annotation(annotation.id.as_str()))
                .is_none_or(|thread| !app.threads.is_running(&thread.id))
        })
        .map(|annotation| annotation.id.clone())
        .collect()
}

/// Sends what [`fixable`] finds, one run a thread.
pub(super) fn run(app: &mut App, target: &FixTarget, effects: &mut Vec<Effect>) {
    let tab = comments::tab(app);
    let mut threads: Vec<(ThreadId, AnnotationId)> = Vec::new();
    for id in fixable(app, target) {
        let found = app.threads.thread_of_annotation(id.as_str());
        let thread = if let Some(thread) = found.map(|thread| thread.id.clone()) {
            thread
        } else {
            let text = (app.document.annotation(&id))
                .map(|annotation| annotation.text.clone())
                .unwrap_or_default();
            let (stamp, fresh) = (now(app), send::new_thread_id(app));
            let message = format!("tmsg_{}", app.fresh_id());
            let (thread, changed) =
                (app.threads).queue_comment(&tab, fresh, &message, id.as_str(), &text, &stamp);
            effects::emit(&changed, effects);
            thread
        };
        if !threads.iter().any(|(held, _)| *held == thread) {
            threads.push((thread, id));
        }
    }
    for (thread, id) in threads {
        let started = send::begin(app, &thread, Some(id.as_str()));
        effects::run_started(app, started, specular_agent::Changed::default(), effects);
    }
}
