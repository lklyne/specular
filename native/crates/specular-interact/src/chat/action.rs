//! [`ChatAction`]: what the panel's controls ask for.

use specular_agent::{MediaType, ThreadId};

use super::{comments, effects, send};
use crate::{App, AssetBytes, Effect, iso8601};

/// An image pasted into the composer.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageUpload {
    /// The format.
    pub media_type: MediaType,
    /// The file's contents.
    pub bytes: AssetBytes,
}

/// Something done in the right panel.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatAction {
    /// Show the panel, or hide it.
    Toggle,
    /// Make the panel this wide, in logical pixels, within its limits.
    Resize(f32),
    /// Start an empty thread on the active canvas and open it.
    NewThread,
    /// Open one of the active canvas's threads.
    SelectThread(ThreadId),
    /// Go back to the list of threads. Nothing is archived.
    Back,
    /// Archive a thread, which drops out of the list and stays on disk.
    /// `None` is the open one. Refused while the agent works on it.
    CloseThread(Option<ThreadId>),
    /// Send what the composer holds. With a comment draft open that is the
    /// comment: it is saved and queued, and the agent does not run.
    /// Otherwise the text and images join the open thread's queue, and the
    /// agent is run on the queue, or once the run in flight ends. With no
    /// text and no images it sends the queue as it is.
    Send {
        /// The words typed.
        text: String,
        /// The images pasted.
        images: Vec<ImageUpload>,
    },
    /// Drop the comment draft; nothing is saved.
    DropDraft,
    /// Stop the open thread's run.
    Stop,
    /// Resolve every open comment of the open thread that was sent, as one
    /// undo step.
    ResolveComments,
}

/// Runs `action`.
pub(crate) fn run(app: &mut App, action: ChatAction, effects: &mut Vec<Effect>) {
    match action {
        ChatAction::Toggle => app.session.chat.toggle(),
        ChatAction::Resize(width) => app.session.chat.resize(width),
        ChatAction::NewThread => {
            let (tab, now, id) = (comments::tab(app), now(app), send::new_thread_id(app));
            let changed = app.threads.new_thread(&tab, id, &now);
            effects::emit(&changed, effects);
        }
        ChatAction::SelectThread(id) => {
            let changed = app.threads.select(&comments::tab(app), &id);
            effects::emit(&changed, effects);
        }
        ChatAction::Back => {
            let changed = app.threads.deselect(&comments::tab(app));
            effects::emit(&changed, effects);
        }
        ChatAction::CloseThread(id) => {
            let tab = comments::tab(app);
            let target = id.or_else(|| app.threads.active(&tab).map(|t| t.id.clone()));
            if let Some(id) = target {
                let changed = app.threads.close(&id, &now(app));
                effects::emit(&changed, effects);
            }
        }
        ChatAction::Send { text, images } => send::run(app, &text, images, effects),
        ChatAction::DropDraft => {
            if app.session.comment_draft.is_some() {
                crate::comment::cancel(app, effects);
            }
        }
        ChatAction::Stop => {
            let active = app
                .threads
                .active(&comments::tab(app))
                .map(|t| t.id.clone());
            if let Some(id) = active.filter(|id| app.threads.is_running(id)) {
                effects.push(Effect::CancelAgent(id));
            }
        }
        ChatAction::ResolveComments => comments::resolve_sent(app, effects),
    }
}

/// The time threads are stamped with.
pub(super) fn now(app: &App) -> String {
    iso8601(app.session.now_ms)
}
