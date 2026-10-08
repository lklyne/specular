//! Send: the composer's text and images, as a comment or as a message.

use specular_agent::{Image, MediaType, Started, ThreadId};

use super::action::{ImageUpload, now};
use super::{comments, commit, effects, pill};
use crate::{App, Effect, TextEdit};

/// An id for a thread nothing uses.
pub(super) fn new_thread_id(app: &mut App) -> ThreadId {
    ThreadId(format!("thread_{}", app.fresh_id()))
}

/// The file extension of an image of `media_type`.
fn extension(media_type: MediaType) -> &'static str {
    match media_type {
        MediaType::Png => "png",
        MediaType::Jpeg => "jpg",
        MediaType::Gif => "gif",
        MediaType::Webp => "webp",
    }
}

/// Writes each image beside the thread, under
/// `.specular/threads/<canvas>/attachments/<thread>/`, and returns how the
/// message records them. The writes come before any effect that reads them.
pub(super) fn store_images(
    app: &mut App,
    tab: &str,
    thread: &ThreadId,
    uploads: Vec<ImageUpload>,
    effects: &mut Vec<Effect>,
) -> Vec<Image> {
    uploads
        .into_iter()
        .map(|upload| {
            let file = format!(
                ".specular/threads/{tab}/attachments/{thread}/img_{}.{}",
                app.fresh_id(),
                extension(upload.media_type),
                thread = thread.as_str(),
            );
            effects.push(Effect::WriteAsset {
                file: file.clone(),
                bytes: upload.bytes,
            });
            Image {
                path: file,
                media_type: upload.media_type,
            }
        })
        .collect()
}

/// What Send does.
pub(super) fn run(app: &mut App, text: &str, images: Vec<ImageUpload>, effects: &mut Vec<Effect>) {
    if let Some(draft) = app.session.comment_draft.clone() {
        if commit::comment(app, draft, text, images, effects) {
            app.session.comment_draft = None;
            if app
                .session
                .editing
                .as_ref()
                .is_some_and(TextEdit::is_comment)
            {
                app.session.editing = None;
                effects.push(Effect::SetImeAllowed(false));
            }
        }
        return;
    }
    send_thread(app, text, images, effects);
}

/// `sendActiveThread`: the composer's words join the open thread's queue
/// (starting a draft when none is open), and the queue is sent.
fn send_thread(app: &mut App, text: &str, images: Vec<ImageUpload>, effects: &mut Vec<Effect>) {
    let tab = comments::tab(app);
    let now = now(app);
    let fresh = new_thread_id(app);
    let target = comments::active_id(app).unwrap_or_else(|| fresh.clone());
    let has_content = !text.trim().is_empty() || !images.is_empty();
    // A send with nothing in it only sends what is queued, and needs a thread.
    if !has_content && comments::active_thread(app).is_none() {
        return;
    }
    let stored = store_images(app, &tab, &target, images, effects);
    let message = format!("tmsg_{}", app.fresh_id());
    let queued = app
        .threads
        .queue_message(&tab, fresh, &message, text, stored, &now);
    let Some((thread, changed)) = queued else {
        return;
    };
    let started = begin(app, &thread, None);
    effects::run_started(started, changed, effects);
}

/// Sends the queue of `thread` if no run is in flight. `aim` is the pin the
/// turn is about; without one the live pill is.
pub(super) fn begin(app: &mut App, thread: &ThreadId, aim: Option<&str>) -> Option<Started> {
    let now = now(app);
    let aimed = aim.and_then(|id| pill::of_comment(app, id));
    let pill = aimed.unwrap_or_else(|| pill::live(app));
    let ctx = super::prompt::context(app, thread, pill);
    app.threads.begin_run(thread, &now, &ctx)
}
