//! The comment verbs: `annotations`, `annotation`, `annotate`,
//! `annotate-selection`, `ack`, `resolve`, `dismiss` and `reply`.
//!
//! The list is cut down to what an agent reads first, and the detail moves
//! the heavy parts out of the JSON: the elements as lines of text and the
//! region's picture as a file.

use std::fmt::Write as _;

use base64::Engine as _;
use serde_json::{Map, Value, json};

use crate::args::Args;
use crate::client::Client;
use crate::output::{self, Block};
use crate::usage;

/// `text` as a query string writes it.
fn form_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b' ' => out.push('+'),
            b'-' | b'.' | b'_' | b'*' => out.push(char::from(byte)),
            _ if byte.is_ascii_alphanumeric() => out.push(char::from(byte)),
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

fn array<'v>(value: &'v Value, key: &str) -> &'v [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// What a region comment or an element comment is about, in a line.
fn summary(annotation: &Value, metadata: &Value) -> Option<Value> {
    if annotation["anchor"]["type"] == "region" {
        let elements = array(metadata, "regionElements");
        let components = array(metadata, "regionComponents");
        let count = |groups: &[Value], key: &str| -> usize {
            groups.iter().map(|group| array(group, key).len()).sum()
        };
        return Some(json!({
            "type": "region",
            "pageCount": elements.len(),
            "elementCount": count(elements, "elements"),
            "hasScreenshot": metadata["regionScreenshot"].as_str().is_some_and(|s| !s.is_empty()),
            "componentCount": count(components, "components"),
        }));
    }
    let context = metadata.get("inspectContext").filter(|c| c.is_object())?;
    let or_null = |key: &str| context.get(key).cloned().unwrap_or(Value::Null);
    Some(json!({
        "type": "element",
        "tagName": or_null("tagName"),
        "name": or_null("name"),
        "cssClasses": context.get("cssClasses").cloned().unwrap_or_else(|| json!([])),
        "textPreview": or_null("textPreview"),
    }))
}

fn slim(annotation: &Value) -> Value {
    let mut out = Map::new();
    let keys = [
        "id",
        "anchor",
        "author",
        "text",
        "status",
        "createdAt",
        "replies",
    ];
    for key in keys {
        if let Some(value) = annotation.get(key) {
            out.insert(key.to_owned(), value.clone());
        }
    }
    if let Some(page) = annotation.get("pageAnchor").filter(|page| truthy(page)) {
        out.insert("pageAnchor".to_owned(), page.clone());
    }
    let metadata = &annotation["metadata"];
    let mut meta = Map::new();
    if let Some(name) = metadata.get("pageName").filter(|name| truthy(name)) {
        meta.insert("pageName".to_owned(), name.clone());
    }
    if let Some(summary) = summary(annotation, metadata) {
        meta.insert("summary".to_owned(), summary);
    }
    if !meta.is_empty() {
        out.insert("metadata".to_owned(), Value::Object(meta));
    }
    Value::Object(out)
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|n| n != 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `annotations [--status S | --all] [--url U] [--page-id P]`.
pub(crate) fn list(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let status = if args.switch("all") {
        "all"
    } else {
        args.flag("status").unwrap_or("unresolved")
    };
    let mut query = Vec::new();
    if !status.is_empty() {
        query.push(format!("status={}", form_encode(status)));
    }
    for (flag, key) in [("url", "url"), ("page-id", "page_id")] {
        if let Some(value) = args.flag(flag).map(str::trim).filter(|v| !v.is_empty()) {
            query.push(format!("{key}={}", form_encode(value)));
        }
    }
    let path = if query.is_empty() {
        "/annotations".to_owned()
    } else {
        format!("/annotations?{}", query.join("&"))
    };
    let answer = client.get(&path)?;
    let stubs: Vec<Value> = array(&answer, "annotations").iter().map(slim).collect();
    output::json(&json!({ "annotations": stubs }));
    Ok(0)
}

fn js_text(value: Option<&Value>, fallback: &str) -> String {
    match value {
        None | Some(Value::Null) => fallback.to_owned(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}

/// The region's elements as lines: `- <tag>.class "text"`.
fn elements_text(groups: &[Value]) -> String {
    let mut text = "Elements in region:".to_owned();
    for group in groups {
        let page = js_text(group.get("pageName"), "undefined");
        let _ = write!(text, "\n  Page \"{page}\":");
        for element in array(group, "elements") {
            let tag = js_text(element.get("tagName"), "?");
            let classes: Vec<String> = array(element, "cssClasses")
                .iter()
                .map(|class| js_text(Some(class), ""))
                .collect();
            let classes = classes.join(".");
            let dotted = if classes.is_empty() {
                String::new()
            } else {
                format!(".{classes}")
            };
            let preview = js_text(element.get("textPreview"), "");
            let _ = write!(text, "\n    - <{tag}>{dotted} \"{preview}\"");
        }
    }
    text
}

/// The annotation as JSON with its picture, elements and empty component
/// lists taken out, and its inspect context without what the anchor says.
fn detail_blocks(mut annotation: Value, screenshot: bool) -> Vec<Block> {
    let metadata = annotation["metadata"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let picture = metadata
        .get("regionScreenshot")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let elements = metadata
        .get("regionElements")
        .and_then(Value::as_array)
        .cloned();
    let components = metadata.get("regionComponents").cloned();
    let mut detail = metadata;
    for key in ["regionScreenshot", "regionElements", "regionComponents"] {
        detail.shift_remove(key);
    }
    if let Some(Value::Object(context)) = detail.get_mut("inspectContext") {
        for key in ["id", "nodeId", "timestamp"] {
            context.shift_remove(key);
        }
        let anchor = &annotation["anchor"];
        if anchor["type"] == "element" {
            for key in ["elementPath", "boundingBox"] {
                if truthy(&anchor[key]) {
                    context.shift_remove(key);
                }
            }
        }
    }
    let has_components = (components.as_ref())
        .and_then(Value::as_array)
        .is_some_and(|groups| {
            groups
                .iter()
                .any(|group| !array(group, "components").is_empty())
        });
    if has_components && let Some(components) = components {
        detail.insert("regionComponents".to_owned(), components);
    }
    annotation["metadata"] = Value::Object(detail);
    let text = serde_json::to_string_pretty(&annotation).unwrap_or_default();
    let mut blocks = vec![Block::Text(text)];
    if let Some(groups) = elements.filter(|groups| !groups.is_empty()) {
        blocks.push(Block::Text(elements_text(&groups)));
    }
    if screenshot
        && let Some(data) = picture
            .filter(|picture| !picture.is_empty())
            .and_then(|picture| {
                base64::engine::general_purpose::STANDARD
                    .decode(picture)
                    .ok()
            })
    {
        blocks.push(Block::Image { data, jpeg: false });
    }
    blocks
}

/// `annotation <id> [--no-screenshot]`.
pub(crate) fn detail(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let Some(id) = args.nth(0) else {
        return usage("specular annotation <id>");
    };
    let annotation = client.get(&format!("/annotations/{id}"))?;
    output::blocks(&detail_blocks(annotation, !args.switch("no-screenshot")))?;
    Ok(0)
}

/// `annotate <text> [--page-id P]`: a comment on the page, else on the
/// view.
pub(crate) fn annotate(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let text = args.words(0);
    if text.is_empty() {
        return usage("specular annotate <text>");
    }
    let anchor = match args.flag("page-id") {
        Some(page) => json!({ "type": "page", "pageId": page }),
        None => json!({ "type": "viewport" }),
    };
    let body = json!({ "text": text, "anchor": anchor, "author": "agent" });
    output::json(&client.post("/annotations", &body)?);
    Ok(0)
}

/// `annotate-selection <text> [--ids a,b]`: one region comment over these
/// entities, or the selection.
pub(crate) fn annotate_selection(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let text = args.words(0);
    if text.is_empty() {
        return usage(
            "specular annotate-selection \"<text>\" [--ids id1,id2]  (no ids uses current \
             selection)",
        );
    }
    let ids: Vec<&str> = (args.flag("ids").unwrap_or_default().split(','))
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .collect();
    let mut body = json!({ "text": text });
    if !ids.is_empty() {
        body["entityIds"] = json!(ids);
    }
    output::json(&client.post("/selection/annotate", &body)?);
    Ok(0)
}

/// `ack`, `resolve`, `dismiss` and `reply`, which answer a comment.
pub(crate) fn respond(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let verb = args.verb.as_str();
    let id = args.nth(0);
    let (route, body) = match (verb, id) {
        ("reply", Some(_)) if !args.words(1).is_empty() => {
            ("reply", json!({ "author": "agent", "text": args.words(1) }))
        }
        ("reply", _) => return usage("specular reply <annotation-id> <text>"),
        (_, None) => return usage(&format!("specular {verb} <annotation-id>")),
        ("ack", Some(_)) => ("acknowledge", json!({})),
        ("dismiss", Some(_)) => (
            "dismiss",
            args.flag("reason")
                .map_or_else(|| json!({}), |reason| json!({ "reason": reason })),
        ),
        (_, Some(_)) => ("resolve", json!({})),
    };
    let id = id.unwrap_or_default();
    output::json(&client.post(&format!("/annotations/{id}/{route}"), &body)?);
    Ok(0)
}
