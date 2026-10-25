//! The `comments` and `chat` topics.

use std::fmt::Write as _;

use specular_doc::{AnnotationAnchor, AnnotationId, RegionAnchor};
use specular_interact::panel::toolbar;
use specular_interact::{App, ArtifactCard, MarkShape, ThreadRole, chat};

use super::{n, rect, screen, yes};

fn anchor(anchor: &AnnotationAnchor) -> String {
    match anchor {
        AnnotationAnchor::Canvas { canvas_x, canvas_y } => {
            format!("canvas-point {} {}", n(*canvas_x), n(*canvas_y))
        }
        AnnotationAnchor::Page {
            page_id,
            offset_x,
            offset_y,
        } => format!("page-point {page_id} {} {}", n(*offset_x), n(*offset_y)),
        AnnotationAnchor::Element {
            page_id, selector, ..
        } => format!("element {page_id} {selector}"),
        AnnotationAnchor::Region(RegionAnchor::Canvas { canvas_rect }) => {
            format!("canvas-region {}", rect(*canvas_rect))
        }
        AnnotationAnchor::Region(RegionAnchor::Document { doc_rect }) => {
            format!("page-region {}", rect(*doc_rect))
        }
    }
}

pub(super) fn comments(app: &App, out: &mut Vec<String>) {
    for mark in app.comment_marks() {
        let (shape, at) = match mark.shape {
            MarkShape::Badge(pill) => ("pill", screen(pill)),
            MarkShape::Region(frame) => ("region", screen(frame)),
        };
        let mut line = format!("mark {} {shape} {at}", mark.annotation);
        if mark.focused {
            line.push_str(" focused");
        }
        if mark.count > 1 {
            let _ = write!(line, " count {}", mark.count);
        }
        if mark.members.len() > 1 {
            let members: Vec<&str> = mark.members.iter().map(AnnotationId::as_str).collect();
            let _ = write!(line, " members {}", members.join(","));
        }
        out.push(line);
    }
    if let Some(focused) = app.focused_comment() {
        out.push(format!("focus {focused}"));
    }
    if let Some(draft) = app.comment_draft() {
        out.push(format!("draft {}", anchor(&draft.anchor)));
        let page = (draft.page_anchor.as_ref()).map_or("none", |page| page.page_id.as_str());
        out.push(format!("draft page {page}"));
    }
    if let Some(card) = app.comment_composer() {
        out.push(format!("composer {}", rect(card)));
    }
    if let Some(preview) = app.session().comment_preview() {
        out.push(format!("preview {}", rect(preview)));
    }
    for outline in app.comment_highlights() {
        out.push(format!("highlight {}", screen(outline)));
    }
    for annotation in app.document().annotations() {
        let status = format!("{:?}", annotation.status).to_lowercase();
        let mut line = format!(
            "comment {} {status} {} \"{}\"",
            annotation.id,
            anchor(&annotation.anchor),
            annotation.text
        );
        let metadata = annotation.metadata.as_ref();
        if let Some(thread) = metadata
            .and_then(|metadata| metadata.get("threadId"))
            .and_then(|id| id.as_str())
        {
            let _ = write!(line, " thread {thread}");
        }
        if let Some(page) = &annotation.page_anchor {
            let _ = write!(line, " on {}", page.page_id);
        }
        out.push(line);
        if let Some(metadata) = metadata {
            let text = serde_json::to_string(metadata).unwrap_or_default();
            out.push(format!("comment {} metadata {text}", annotation.id));
        }
    }
}

fn cards(cards: &[ArtifactCard], out: &mut Vec<String>) {
    for card in cards {
        let verb = if card.created { "new" } else { "edited" };
        let live = if card.focus.is_some() { " live" } else { "" };
        out.push(format!(
            "card \"{}\" {} {verb}{live}",
            card.label, card.detail
        ));
    }
}

pub(super) fn chat_lines(app: &App, out: &mut Vec<String>) {
    let model = chat(app);
    out.push(format!(
        "chat available={} visible={} width={} title=\"{}\"",
        yes(model.available),
        yes(model.visible),
        model.width,
        model.title
    ));
    for row in &model.threads {
        let mut line = format!("thread \"{}\"", row.title);
        for (on, word) in [
            (row.active, " active"),
            (row.draft, " draft"),
            (row.running, " running"),
        ] {
            if on {
                line.push_str(word);
            }
        }
        out.push(line);
    }
    out.push(format!(
        "controls back={} close={}",
        yes(model.back.is_some()),
        yes(model.close.is_some())
    ));
    match &model.transcript {
        Some(transcript) => transcript_lines(transcript, out),
        None => out.push("list".to_owned()),
    }
    let composer = &model.composer;
    if let Some(draft) = &composer.draft {
        out.push(format!("draft {:?} \"{}\"", draft.kind, draft.label));
    }
    if let Some(open) = &composer.open_comments {
        out.push(format!("open-comments {}", open.count));
    }
    for chip in &composer.queued {
        let pinned = if chip.annotation.is_some() {
            " pinned"
        } else {
            ""
        };
        out.push(format!("queued \"{}\"{pinned}", chip.text));
    }
    out.push(format!("placeholder \"{}\"", composer.placeholder));
    out.push(format!("send-empty {}", yes(composer.can_send_empty)));
    out.push(format!("running {}", yes(composer.running)));
    out.push(format!(
        "pill {:?} \"{}\"",
        composer.pill.kind, composer.pill.label
    ));
    if let Some(folder) = &composer.folder {
        out.push(format!("folder {folder}"));
    }
    if let Some(auto) = &composer.auto {
        out.push(format!("auto {} {}", auto.origin, yes(auto.on)));
    }
    out.push(format!("model {}", composer.model.label));
    for option in &composer.model.options {
        let chosen = if option.chosen { " chosen" } else { "" };
        out.push(format!("model-option \"{}\"{chosen}", option.name));
    }
    if let Some(button) = toolbar(app).chat {
        let open = if button.open { "open" } else { "closed" };
        out.push(format!("toggle {open} \"{}\"", button.label));
    }
    for thread in app.threads().all() {
        let status = format!("{:?}", thread.status).to_lowercase();
        let session = thread.claude_session_id.as_deref().unwrap_or("none");
        out.push(format!(
            "stored \"{}\" {status} messages {} session {session}",
            thread.title,
            thread.messages.len()
        ));
    }
}

fn transcript_lines(transcript: &specular_interact::Transcript, out: &mut Vec<String>) {
    for bubble in &transcript.messages {
        let role = match bubble.role {
            ThreadRole::User => "user",
            ThreadRole::Agent => "agent",
        };
        let mut line = format!("bubble {role} \"{}\"", bubble.text);
        if bubble.annotation.is_some() {
            line.push_str(" pinned");
        }
        if bubble.focused {
            line.push_str(" focused");
        }
        if bubble.focus.is_some() {
            line.push_str(" live");
        }
        out.push(line);
        for image in &bubble.images {
            out.push(format!("image {image}"));
        }
        cards(&bubble.artifacts, out);
    }
    if let Some(text) = &transcript.streaming {
        out.push(format!("streaming \"{text}\""));
    }
    cards(&transcript.making, out);
    if let Some(run) = &transcript.run {
        out.push(format!("run \"{}\"", run.label));
        for line in &run.log {
            out.push(format!("log \"{line}\""));
        }
    }
    if let Some(error) = &transcript.error {
        out.push(format!("error \"{error}\""));
    }
    if let Some(hint) = &transcript.empty_hint {
        out.push(format!("hint \"{hint}\""));
    }
}
