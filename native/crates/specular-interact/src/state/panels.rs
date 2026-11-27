//! The sidebar, the right panel and the canvas as keys (ADR 0059): what a
//! mod needs to draw them again. A row or a control is written with the id
//! `control.press {id}` presses; the right panel's acts are `chat.*`
//! commands. No rect is in any of them, so a drag moves no key.

use serde_json::{Value, json};
use specular_agent::Role;
use specular_doc::{EntityId, ItemId, Kind};

use crate::{App, ChatModel, SectionHead, SidebarModel, SidebarRow, chat, sidebar};

fn head(head: &SectionHead) -> Value {
    json!({ "id": head.id, "title": head.title, "folded": head.folded })
}

fn row(row: &SidebarRow) -> Value {
    json!({
        "id": row.id,
        "label": row.label,
        "icon": row.glyph,
        "trailing": row.trailing,
        "selected": row.selected,
        "dimmed": row.dimmed,
        "expanded": row.expanded,
        "children": row.children.iter().map(self::row).collect::<Vec<_>>(),
    })
}

fn section(head: &SectionHead, rows: &[SidebarRow]) -> Value {
    let mut out = self::head(head);
    out["rows"] = rows.iter().map(row).collect();
    out
}

/// The `panels/sidebar` key.
pub(super) fn sidebar_json(app: &App) -> Value {
    let SidebarModel {
        visible,
        canvases_head,
        canvases,
        notes_head,
        notes,
        pages_head,
        pages,
        ..
    } = sidebar(app);
    let canvases: Vec<Value> = (canvases.iter())
        .map(|canvas| {
            json!({
                "id": canvas.control,
                "canvas": canvas.id.as_str(),
                "label": canvas.label,
                "active": canvas.active,
                "count": canvas.entity_count,
                "rename": canvas.rename.id,
            })
        })
        .collect();
    let mut canvas_section = head(&canvases_head);
    canvas_section["rows"] = Value::Array(canvases);
    json!({
        "visible": visible,
        "add": "sidebar.add",
        "sections": [canvas_section, section(&notes_head, &notes), section(&pages_head, &pages)],
    })
}

const fn role(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Agent => "agent",
    }
}

/// The `panels/chat` key.
pub(super) fn chat_json(app: &App) -> Value {
    let ChatModel {
        available,
        visible,
        width,
        title,
        threads,
        transcript,
        composer,
        back,
        ..
    } = chat(app);
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
    let transcript = transcript.map(|transcript| {
        let messages: Vec<Value> = (transcript.messages.iter())
            .map(|bubble| {
                json!({
                    "id": bubble.id,
                    "role": role(bubble.role),
                    "text": bubble.text,
                    "images": bubble.images,
                })
            })
            .collect();
        json!({
            "messages": messages,
            "streaming": transcript.streaming,
            "run": transcript.run.map(|run| json!({ "label": run.label, "log": run.log })),
            "error": transcript.error,
            "emptyHint": transcript.empty_hint,
        })
    });
    let models: Vec<Value> = (composer.model.options.iter())
        .map(|option| {
            json!({ "name": option.name, "description": option.description, "chosen": option.chosen })
        })
        .collect();
    json!({
        "available": available,
        "visible": visible,
        "width": width,
        "title": title,
        "canGoBack": back.is_some(),
        "threads": threads,
        "transcript": transcript,
        "composer": {
            "placeholder": composer.placeholder,
            "running": composer.running,
            "canSendEmpty": composer.can_send_empty,
            "model": { "label": composer.model.label, "options": models },
        },
    })
}

/// The `selection` key: the entities and the edges selected.
pub(super) fn selection_json(app: &App) -> Value {
    let (mut ids, mut edges) = (Vec::new(), Vec::new());
    for item in app.session.selection.items() {
        match item {
            ItemId::Entity(id) => ids.push(id.as_str().to_owned()),
            ItemId::Edge(id) => edges.push(id.as_str().to_owned()),
        }
    }
    json!({ "ids": ids, "edges": edges })
}

/// What an item says of itself beyond its kind: a page's address, a
/// text's or a shape's words and form, a mod item's mod.
fn detail(kind: &Kind) -> Value {
    match kind {
        Kind::Page(page) => json!({ "url": page.url }),
        Kind::Text(text) => json!({ "text": text.text }),
        Kind::Shape(shape) => json!({ "shape": shape.shape, "text": shape.text }),
        Kind::Mod(item) => json!({ "mod": item.name }),
        Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => json!({}),
    }
}

/// The `canvas/items` key: every item of the active canvas, back to
/// front, without its place or size.
pub(super) fn items_json(app: &App) -> Value {
    let items = app.document.entities().map(|entity| {
        let mut item = json!({
            "id": entity.id.as_str(),
            "kind": entity.kind.name(),
            "label": entity.label,
            "parent": entity.parent.as_ref().map(EntityId::as_str),
        });
        if let (Value::Object(item), Value::Object(detail)) = (&mut item, detail(&entity.kind)) {
            item.extend(detail);
        }
        item
    });
    Value::Array(items.collect())
}

/// The `canvas/info` key: the canvas showing, and every canvas of the space.
pub(super) fn info_json(app: &App) -> Value {
    let active = app.space.active();
    let canvases: Vec<Value> = (app.space.canvases().iter())
        .map(|canvas| json!({ "id": canvas.id.as_str(), "name": canvas.name, "active": canvas.is_active() }))
        .collect();
    json!({
        "id": active.id.as_str(),
        "name": active.name,
        "items": app.document.entities().count(),
        "edges": app.document.edges().count(),
        "canvases": canvases,
    })
}
