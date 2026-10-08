//! Saving a comment: the one place a draft becomes an annotation.
//!
//! The on-canvas composer and the panel's Send both end here. The comment is
//! queued into a thread, and the thread's id is written on the annotation in
//! the same undo step that inserts it, so undo takes the comment whole. The
//! thread keeps its message: chat is not undoable.

use serde_json::Value;
use specular_agent::ThreadId;
use specular_doc::{Annotation, Command, JsonMap};

use super::action::{ImageUpload, now};
use super::{comments, effects, send};
use crate::gesture::apply_step;
use crate::{App, Effect};

/// Saves `draft` with `text`, trimmed, and queues it. Nothing happens, and
/// `false` comes back, for no text and no images. The comment takes the
/// focus, as a pin placed does.
pub(crate) fn comment(
    app: &mut App,
    draft: Annotation,
    text: &str,
    images: Vec<ImageUpload>,
    effects: &mut Vec<Effect>,
) -> bool {
    let text = text.trim();
    if text.is_empty() && images.is_empty() {
        return false;
    }
    let tab = comments::tab(app);
    let (now, fresh) = (now(app), send::new_thread_id(app));
    let id = draft.id.clone();
    let home = app.threads.comment_thread(&tab, id.as_str(), &fresh);
    let stored = send::store_images(app, &tab, &home, images, effects);
    let message = format!("tmsg_{}", app.fresh_id());
    let (thread, changed) = app.threads.queue_comment_with_images(
        &tab,
        fresh,
        &message,
        id.as_str(),
        text,
        stored,
        &now,
    );
    effects::emit(&changed, effects);
    let annotation = Annotation {
        text: text.to_owned(),
        metadata: Some(with_thread(draft.metadata.clone(), &thread)),
        ..draft
    };
    let command = Command::InsertAnnotation {
        annotation: Box::new(annotation),
        at: app.document.annotations().len(),
    };
    apply_step(app, command);
    if app.document.annotation(&id).is_some() {
        app.session.focused_comment = Some(id);
        app.session.selection.set([]);
    }
    true
}

fn with_thread(metadata: Option<JsonMap>, thread: &ThreadId) -> JsonMap {
    let mut metadata = metadata.unwrap_or_default();
    metadata.insert(
        "threadId".to_owned(),
        Value::String(thread.as_str().to_owned()),
    );
    metadata
}
