//! The agent threads as keys (ADR 0059): the right panel as it draws
//! (`panels/chat`), the active canvas's threads (`chat/threads`), and each
//! of those threads on its own (`chat/thread/<id>`), so a mod can show any
//! thread in a block, a canvas item or a pane of its own, beside the panel.
//!
//! `chat/threads` holds no transcript, so it moves when a thread is made,
//! renamed, opened or starts or stops working, not on every word streamed.
//! A thread's own key moves only with that thread.

use serde_json::{Value, json};
use specular_agent::{Role, Status, Thread};

use crate::{App, ArtifactCard, ChatModel, ModelChip, PillKind, Transcript, chat};

/// The prefix of a thread's own key; the thread's id follows it.
pub const THREAD_KEY: &str = "chat/thread/";

const fn role(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Agent => "agent",
    }
}

fn cards(cards: &[ArtifactCard]) -> Vec<Value> {
    (cards.iter())
        .map(|card| json!({ "label": card.label, "detail": card.detail, "created": card.created }))
        .collect()
}

fn transcript_json(transcript: &Transcript) -> Value {
    let messages: Vec<Value> = (transcript.messages.iter())
        .map(|bubble| {
            json!({
                "id": bubble.id,
                "role": role(bubble.role),
                "text": bubble.text,
                "images": bubble.images,
                "artifacts": cards(&bubble.artifacts),
            })
        })
        .collect();
    json!({
        "messages": messages,
        "streaming": transcript.streaming,
        "making": cards(&transcript.making),
        "run": (transcript.run.as_ref()).map(|run| json!({ "label": run.label, "log": run.log })),
        "error": transcript.error,
        "emptyHint": transcript.empty_hint,
    })
}

fn model_json(model: &ModelChip) -> Value {
    let options: Vec<Value> = (model.options.iter())
        .map(|option| {
            json!({ "name": option.name, "value": option.value, "description": option.description, "chosen": option.chosen })
        })
        .collect();
    json!({ "label": model.label, "options": options })
}

/// The `panels/chat` key.
pub(super) fn panel_json(app: &App) -> Value {
    let ChatModel {
        available,
        visible,
        width,
        title,
        threads,
        transcript,
        composer,
        back,
        actions,
        ..
    } = chat(app);
    let actions: Vec<Value> = (actions.iter())
        .map(|button| json!({ "command": button.command, "title": button.title, "args": button.args }))
        .collect();
    let threads: Vec<Value> = (threads.iter())
        .map(|thread| {
            json!({
                "id": thread.id.0,
                "title": thread.title,
                "draft": thread.draft,
                "active": thread.active,
                "running": thread.running,
                "updatedAt": thread.updated_at,
            })
        })
        .collect();
    json!({
        "available": available,
        "visible": visible,
        "width": width,
        "title": title,
        "canGoBack": back.is_some(),
        "actions": actions,
        "threads": threads,
        "transcript": transcript.as_ref().map(transcript_json),
        "composer": {
            "placeholder": composer.placeholder,
            "running": composer.running,
            "canSendEmpty": composer.can_send_empty,
            "model": model_json(&composer.model),
        },
    })
}

fn canvas_threads(app: &App) -> Vec<&Thread> {
    app.threads.for_canvas(app.space.active().id.as_str())
}

fn open_id(app: &App) -> Option<&str> {
    let open = app.threads.active(app.space.active().id.as_str());
    open.map(|thread| thread.id.as_str())
}

/// The `chat/threads` key: the active canvas's threads, newest first, which
/// one the panel has open, and the model runs use.
pub(super) fn threads_json(app: &App) -> Value {
    let open = open_id(app);
    let threads: Vec<Value> = (canvas_threads(app).into_iter())
        .map(|thread| {
            json!({
                "id": thread.id.as_str(),
                "key": format!("{THREAD_KEY}{}", thread.id.as_str()),
                "title": thread.title,
                "draft": thread.status == Status::Draft,
                "open": open == Some(thread.id.as_str()),
                "running": app.threads.is_running(&thread.id),
                "updatedAt": thread.updated_at,
            })
        })
        .collect();
    json!({
        "open": open,
        "threads": threads,
        "model": model_json(&crate::chat::model_chip(app)),
    })
}

const fn pill_kind(kind: PillKind) -> &'static str {
    match kind {
        PillKind::Dom => "dom",
        PillKind::Comment => "comment",
        PillKind::Selection => "selection",
        PillKind::Canvas => "canvas",
    }
}

/// The `chat/context` key: what the next message goes with, as the panel's
/// composer shows it under its field: the pill (an element, a comment, the
/// selection or the canvas), the folder the agent writes to, whether
/// comments on its origin send themselves, and the model. Any view's
/// message goes with the same context.
pub(super) fn context_json(app: &App) -> Value {
    let composer = chat(app).composer;
    json!({
        "pill": { "label": composer.pill.label, "kind": pill_kind(composer.pill.kind) },
        "folder": composer.folder,
        "folderPath": composer.folder_path,
        "auto": (composer.auto.as_ref()).map(|auto| json!({ "origin": auto.origin, "on": auto.on })),
        "model": model_json(&composer.model),
    })
}

/// One thread on its own: its conversation, what waits to be sent, and
/// whether its agent works. The panel's comment draft is not in it.
fn thread_json(app: &App, thread: &Thread, open: Option<&str>) -> Value {
    let running = app.threads.is_running(&thread.id);
    let queued: Vec<Value> = (app.threads.queued(&thread.id).into_iter())
        .map(|message| json!(chat::queued_chip(message).text))
        .collect();
    let answered = thread.messages.iter().any(|m| m.role == Role::Agent);
    let placeholder = if running {
        "Queue a follow-up\u{2026}"
    } else if thread.status == Status::Draft || !answered {
        "Add or edit\u{2026}"
    } else {
        "Follow up\u{2026}"
    };
    let mut out = transcript_json(&chat::transcript(app, thread));
    let fields = [
        ("id", json!(thread.id.as_str())),
        ("title", json!(thread.title)),
        ("draft", json!(thread.status == Status::Draft)),
        ("open", json!(open == Some(thread.id.as_str()))),
        ("running", json!(running)),
        ("updatedAt", json!(thread.updated_at)),
        ("queued", json!(queued)),
        ("canSendEmpty", json!(!running && !queued.is_empty())),
        ("placeholder", json!(placeholder)),
    ];
    if let Value::Object(map) = &mut out {
        for (key, value) in fields {
            map.insert(key.to_owned(), value);
        }
    }
    out
}

/// Every `chat/thread/<id>` key of the active canvas, with its value.
pub(super) fn thread_keys(app: &App) -> Vec<(String, Value)> {
    let open = open_id(app);
    (canvas_threads(app).into_iter())
        .map(|thread| {
            let key = format!("{THREAD_KEY}{}", thread.id.as_str());
            (key, thread_json(app, thread, open))
        })
        .collect()
}
