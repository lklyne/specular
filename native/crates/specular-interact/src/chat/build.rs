//! [`chat`]: the panel as it is now.

use specular_agent::{Message, Pill, Role, RunState, Status, Thread};
use specular_doc::AnnotationId;

use super::model::{
    Bubble, ChatModel, Composer, DraftChip, OpenComments, PillChip, PillKind, QueuedChip, RunBar,
    ThreadRow, Transcript,
};
use super::{ChatAction, comments, draft, pill};
use crate::{Action, App, comment};

/// The hint over an empty conversation.
const EMPTY_HINT: &str = "Comment on the canvas to queue a draft, or type below and send.";

/// The right panel for `app` as it is now. Every control carries the
/// [`Action`] it sends; the field's own text is the view's, which sends it
/// with [`ChatAction::Send`].
pub fn chat(app: &App) -> ChatModel {
    let tab = comments::tab(app);
    let view = app.chat_view();
    let active = comments::active_thread(app);
    let act = |action| Action::Chat(action);
    let threads = (app.threads.for_canvas(&tab).into_iter())
        .map(|thread| ThreadRow {
            id: thread.id.clone(),
            title: thread.title.clone(),
            draft: thread.status == Status::Draft,
            active: active.is_some_and(|open| open.id == thread.id),
            updated_at: thread.updated_at.clone(),
            select: act(ChatAction::SelectThread(thread.id.clone())),
            close: act(ChatAction::CloseThread(Some(thread.id.clone()))),
        })
        .collect();
    ChatModel {
        available: view.available(),
        visible: view.shown(),
        width: view.width(),
        toggle: act(ChatAction::Toggle),
        title: active.map_or_else(|| "Threads".to_owned(), |thread| thread.title.clone()),
        threads,
        new_thread: act(ChatAction::NewThread),
        back: active.map(|_| act(ChatAction::Back)),
        close: active.map(|thread| act(ChatAction::CloseThread(Some(thread.id.clone())))),
        transcript: active.map(|thread| transcript(app, thread)),
        composer: composer(app, active),
    }
}

fn transcript(app: &App, thread: &Thread) -> Transcript {
    let run = app.threads.run(&thread.id);
    let running = app.threads.is_running(&thread.id);
    let messages: Vec<Bubble> = (thread.messages.iter())
        .filter(|message| !message.queued)
        .map(|message| bubble(app, message))
        .collect();
    let streaming = run
        .filter(|run| running && !run.text.is_empty())
        .map(|run| run.text.clone());
    let bar = run.filter(|_| running).map(|run| RunBar {
        label: run.current_label().to_owned(),
        log: run.events.iter().map(|event| event.text.clone()).collect(),
        stop: Action::Chat(ChatAction::Stop),
    });
    let error = run.and_then(|run| match &run.state {
        RunState::Failed(error) => Some(error.clone()),
        RunState::Running => None,
    });
    let nothing = messages.is_empty() && bar.is_none() && error.is_none();
    Transcript {
        messages,
        streaming,
        run: bar,
        error,
        empty_hint: nothing.then(|| EMPTY_HINT.to_owned()),
    }
}

fn bubble(app: &App, message: &Message) -> Bubble {
    let annotation = (message.annotation_id.as_deref()).map(AnnotationId::new);
    let live = annotation.as_ref().is_some_and(|id| {
        (app.document.annotation(id)).is_some_and(|annotation| comment::is_shown(app, annotation))
    });
    let folder = app.space.folder();
    Bubble {
        id: message.id.clone(),
        role: message.role,
        text: message.text.clone(),
        focused: annotation.is_some() && app.focused_comment() == annotation.as_ref(),
        focus: annotation
            .clone()
            .filter(|_| live)
            .map(Action::RevealComment),
        annotation,
        images: (message.images.iter())
            .map(|image| match folder {
                Some(folder) => format!("{}/{}", folder.trim_end_matches('/'), image.path),
                None => image.path.clone(),
            })
            .collect(),
    }
}

fn composer(app: &App, active: Option<&Thread>) -> Composer {
    let queued: Vec<&Message> = active
        .map(|thread| app.threads.queued(&thread.id))
        .unwrap_or_default();
    let running = active.is_some_and(|thread| app.threads.is_running(&thread.id));
    let draft_chip = app.comment_draft().map(|draft| {
        let (kind, label) = draft::describe(draft);
        DraftChip {
            label,
            kind,
            remove: Action::Chat(ChatAction::DropDraft),
        }
    });
    let open = active.map(|thread| comments::open_sent(app, thread));
    let is_new = active.is_none_or(|thread| {
        thread.status == Status::Draft || !thread.messages.iter().any(|m| m.role == Role::Agent)
    });
    let placeholder = if draft_chip.is_some() {
        "Add a comment\u{2026}"
    } else if running {
        "Queue a follow-up\u{2026}"
    } else if is_new {
        "Add or edit\u{2026}"
    } else {
        "Follow up\u{2026}"
    };
    Composer {
        can_send_empty: draft_chip.is_none() && !running && !queued.is_empty(),
        draft: draft_chip,
        open_comments: open.filter(|ids| !ids.is_empty()).map(|ids| OpenComments {
            count: ids.len(),
            resolve: Action::Chat(ChatAction::ResolveComments),
        }),
        queued: queued.into_iter().map(queued_chip).collect(),
        placeholder: placeholder.to_owned(),
        pill: pill_chip(app),
        folder: app.space.folder().and_then(last_segment),
        running,
    }
}

fn queued_chip(message: &Message) -> QueuedChip {
    let text = message.text.trim();
    QueuedChip {
        text: if text.is_empty() {
            match message.images.len() {
                1 => "Image".to_owned(),
                count => format!("{count} images"),
            }
        } else {
            text.to_owned()
        },
        annotation: (message.annotation_id.as_deref()).map(AnnotationId::new),
    }
}

fn pill_chip(app: &App) -> PillChip {
    let pill = pill::live(app);
    let kind = match &pill {
        Pill::Dom { .. } => PillKind::Dom,
        Pill::Annotation { .. } => PillKind::Comment,
        Pill::Selection { .. } => PillKind::Selection,
        Pill::Empty => PillKind::Canvas,
    };
    PillChip {
        label: pill.label(&app.space.active().name),
        kind,
    }
}

fn last_segment(path: &str) -> Option<String> {
    path.rsplit('/')
        .find(|part| !part.is_empty())
        .map(str::to_owned)
}
