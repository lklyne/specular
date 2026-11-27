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
    /// Start an empty thread on the active canvas. `id` names it (a fresh
    /// one when `None`; refused when taken), and `open` makes it the
    /// canvas's open thread, as the panel's New does. A view that shows a
    /// thread of its own starts one with `open: false`.
    NewThread {
        /// Its id.
        id: Option<ThreadId>,
        /// Whether it becomes the open thread.
        open: bool,
    },
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
    ///
    /// With `thread` the words go to that thread instead, open or not, and
    /// a comment draft is left alone: a view showing one thread sends there.
    Send {
        /// The words typed.
        text: String,
        /// The images pasted.
        images: Vec<ImageUpload>,
        /// The thread to send to, when not the open one.
        thread: Option<ThreadId>,
    },
    /// Drop the comment draft; nothing is saved.
    DropDraft,
    /// Stop a thread's run: the one named, or the open one.
    Stop(Option<ThreadId>),
    /// Run the agent on this model from the next run on, in every thread.
    SetModel(String),
    /// Resolve every open comment of the open thread that was sent, as one
    /// undo step.
    ResolveComments,
    /// Send comments to the agent to fix, on the origin's repo.
    Fix(super::FixTarget),
}

/// Runs `action`.
pub(crate) fn run(app: &mut App, action: ChatAction, effects: &mut Vec<Effect>) {
    match action {
        ChatAction::Toggle => app.session.chat.toggle(),
        ChatAction::Resize(width) => app.session.chat.resize(width),
        ChatAction::NewThread { id, open } => {
            let (tab, now) = (comments::tab(app), now(app));
            let id = id.unwrap_or_else(|| send::new_thread_id(app));
            let changed = app.threads.new_thread(&tab, id, &now, open);
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
        ChatAction::Send {
            text,
            images,
            thread: Some(thread),
        } => send::to_thread(app, &thread, &text, images, effects),
        ChatAction::Send { text, images, .. } => send::run(app, &text, images, effects),
        ChatAction::DropDraft => {
            if app.session.comment_draft.is_some() {
                crate::comment::cancel(app, effects);
            }
        }
        ChatAction::Stop(thread) => {
            let target =
                thread.or_else(|| (app.threads.active(&comments::tab(app))).map(|t| t.id.clone()));
            if let Some(id) = target.filter(|id| app.threads.is_running(id)) {
                effects.push(Effect::CancelAgent(id));
            }
        }
        ChatAction::SetModel(model) => {
            app.models.choose(&model);
            effects.push(Effect::SaveAgentModel(app.models.chosen().to_owned()));
        }
        ChatAction::ResolveComments => comments::resolve_sent(app, effects),
        ChatAction::Fix(target) => super::fix::run(app, &target, effects),
    }
}

/// The time threads are stamped with.
pub(super) fn now(app: &App) -> String {
    iso8601(app.session.now_ms)
}
