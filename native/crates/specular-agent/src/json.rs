use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::thread::{Image, MediaType, Message, Role, Status, Thread, ThreadId};

fn pretty(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

fn message_json(message: &Message) -> Value {
    let mut map = Map::new();
    map.insert("id".into(), json!(message.id));
    map.insert(
        "role".into(),
        json!(match message.role {
            Role::User => "user",
            Role::Agent => "agent",
        }),
    );
    map.insert("text".into(), json!(message.text));
    map.insert("createdAt".into(), json!(message.created_at));
    if message.queued {
        map.insert("queued".into(), json!(true));
    }
    if let Some(id) = &message.annotation_id {
        map.insert("annotationId".into(), json!(id));
    }
    if !message.images.is_empty() {
        let images: Vec<Value> = message
            .images
            .iter()
            .map(|i| json!({ "path": i.path, "mediaType": i.media_type.as_str() }))
            .collect();
        map.insert("images".into(), Value::Array(images));
    }
    Value::Object(map)
}

impl Thread {
    /// The file text, as Electron's `writeThread` writes it.
    pub fn to_json(&self) -> String {
        let mut map = Map::new();
        map.insert("id".into(), json!(self.id.as_str()));
        map.insert("tabId".into(), json!(self.tab_id));
        map.insert("title".into(), json!(self.title));
        map.insert("status".into(), json!(self.status.as_str()));
        map.insert("createdAt".into(), json!(self.created_at));
        map.insert("updatedAt".into(), json!(self.updated_at));
        if let Some(session) = &self.claude_session_id {
            map.insert("claudeSessionId".into(), json!(session));
        }
        map.insert("annotationIds".into(), json!(self.annotation_ids));
        let messages: Vec<Value> = self.messages.iter().map(message_json).collect();
        map.insert("messages".into(), Value::Array(messages));
        pretty(&Value::Object(map))
    }

    /// Reads a thread file the way Electron's `parseThread` does: bad messages
    /// are skipped and missing fields get defaults. `closed` stays closed.
    /// `None` when the text is not a thread.
    pub fn from_json(text: &str, tab_id_fallback: &str, now: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(text).ok()?;
        let raw = value.as_object()?;
        let id = str_of(raw, "id").filter(|id| !id.is_empty())?;
        let status = match raw.get("status")?.as_str()? {
            "draft" => Status::Draft,
            "open" => Status::Open,
            "closed" => Status::Closed,
            _ => return None,
        };
        let created_at = str_of(raw, "createdAt").unwrap_or(now).to_owned();
        let updated_at = str_of(raw, "updatedAt").map_or_else(|| created_at.clone(), str::to_owned);
        let title = str_of(raw, "title")
            .filter(|t| !t.trim().is_empty())
            .unwrap_or("New thread");
        Some(Self {
            id: ThreadId(id.to_owned()),
            tab_id: str_of(raw, "tabId").unwrap_or(tab_id_fallback).to_owned(),
            title: title.to_owned(),
            status,
            created_at,
            updated_at,
            claude_session_id: str_of(raw, "claudeSessionId").map(str::to_owned),
            annotation_ids: raw
                .get("annotationIds")
                .and_then(Value::as_array)
                .map(|ids| {
                    ids.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            messages: raw
                .get("messages")
                .and_then(Value::as_array)
                .map(|items| items.iter().filter_map(|m| parse_message(m, now)).collect())
                .unwrap_or_default(),
        })
    }
}

fn str_of<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    map.get(key).and_then(Value::as_str)
}

fn parse_message(value: &Value, now: &str) -> Option<Message> {
    let raw = value.as_object()?;
    let role = match str_of(raw, "role")? {
        "user" => Role::User,
        "agent" => Role::Agent,
        _ => return None,
    };
    Some(Message {
        id: str_of(raw, "id")?.to_owned(),
        role,
        text: str_of(raw, "text")?.to_owned(),
        created_at: str_of(raw, "createdAt").unwrap_or(now).to_owned(),
        queued: raw.get("queued") == Some(&Value::Bool(true)),
        annotation_id: str_of(raw, "annotationId").map(str::to_owned),
        images: raw
            .get("images")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(parse_image).collect())
            .unwrap_or_default(),
    })
}

fn parse_image(value: &Value) -> Option<Image> {
    let raw = value.as_object()?;
    Some(Image {
        path: str_of(raw, "path")?.to_owned(),
        media_type: MediaType::parse(str_of(raw, "mediaType")?)?,
    })
}

/// What `.specular/threads/index.json` holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Index {
    /// Electron's single active thread.
    pub active: Option<ThreadId>,
    /// The active thread of each canvas (tab id to thread).
    pub by_canvas: BTreeMap<String, ThreadId>,
}

/// The index file text. `activeByCanvas` is an extension Electron ignores.
pub fn index_json(
    active: Option<&ThreadId>,
    active_by_canvas: &BTreeMap<String, ThreadId>,
) -> String {
    let by_canvas: Map<String, Value> = active_by_canvas
        .iter()
        .map(|(tab, id)| (tab.clone(), json!(id.as_str())))
        .collect();
    pretty(&json!({
        "activeThreadId": active.map(ThreadId::as_str),
        "activeByCanvas": by_canvas,
    }))
}

/// Reads the index file; anything unreadable gives the empty index.
pub fn parse_index(text: &str) -> Index {
    let Ok(Value::Object(raw)) = serde_json::from_str::<Value>(text) else {
        return Index::default();
    };
    Index {
        active: str_of(&raw, "activeThreadId").map(|id| ThreadId(id.to_owned())),
        by_canvas: raw
            .get("activeByCanvas")
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .filter_map(|(tab, id)| Some((tab.clone(), ThreadId(id.as_str()?.to_owned()))))
                    .collect()
            })
            .unwrap_or_default(),
    }
}
