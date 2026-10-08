//! Short, human-readable lines for the messages of a `claude` stream.

use serde_json::{Map, Value};

use crate::stream::{Progress, ProgressKind};
use crate::text::truncate;

struct Block {
    kind: ProgressKind,
    text: String,
    label: String,
}

pub(crate) fn progress(kind: ProgressKind, text: &str, label: Option<String>) -> Progress {
    Progress {
        kind,
        text: truncate(text, 320),
        label,
    }
}

/// The progress line of an assistant message, `None` when no block says anything.
pub(crate) fn assistant(content: &[Value]) -> Option<Progress> {
    let blocks: Vec<Block> = content.iter().filter_map(block).collect();
    let first = blocks.first()?;
    let last = blocks.last()?;
    let merged: Vec<&str> = blocks.iter().map(|b| b.text.as_str()).collect();
    Some(progress(
        first.kind,
        &merged.join(" | "),
        Some(last.label.clone()),
    ))
}

/// The progress line of a user message carrying tool results.
pub(crate) fn user(content: &[Value]) -> Option<Progress> {
    let results: Vec<String> = content
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_result"))
        .map(tool_result)
        .collect();
    if results.is_empty() {
        return None;
    }
    Some(progress(
        ProgressKind::ToolResult,
        &results.join(" | "),
        None,
    ))
}

/// The progress line of a `result` message.
pub(crate) fn result(final_text: &str, subtype: &str) -> Progress {
    let last_line = final_text
        .lines()
        .rfind(|l| !l.trim().is_empty())
        .unwrap_or("");
    let summary = if final_text.is_empty() {
        subtype.to_owned()
    } else {
        truncate(last_line, 200)
    };
    progress(ProgressKind::Result, &summary, Some("Wrapping up".into()))
}

fn block(block: &Value) -> Option<Block> {
    match block.get("type")?.as_str()? {
        "text" => {
            let raw = block.get("text").and_then(Value::as_str).unwrap_or("");
            let text = raw.trim();
            (!text.is_empty()).then(|| Block {
                kind: ProgressKind::Text,
                text: truncate(text, 240),
                label: first_sentence(raw),
            })
        }
        "tool_use" => {
            let name = block.get("name").and_then(Value::as_str).unwrap_or("tool");
            let input = block.get("input");
            Some(Block {
                kind: ProgressKind::ToolUse,
                text: summarize_input(name, input),
                label: tool_label(name, input),
            })
        }
        "thinking" => {
            let thinking = block.get("thinking").and_then(Value::as_str).unwrap_or("");
            (!thinking.is_empty()).then(|| Block {
                kind: ProgressKind::Text,
                text: format!("(thinking) {}", truncate(thinking, 180)),
                label: "Thinking".into(),
            })
        }
        _ => None,
    }
}

fn pick<'a>(record: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .filter_map(|k| record.get(*k).and_then(Value::as_str))
        .find(|v| !v.trim().is_empty())
}

fn tool_label(name: &str, input: Option<&Value>) -> String {
    let empty = Map::new();
    let record = input.and_then(Value::as_object).unwrap_or(&empty);
    let file = |verb: &str| match pick(record, &["file_path", "notebook_path", "path", "filePath"])
    {
        Some(f) => format!("{verb} {}", basename(f)),
        None => verb.to_owned(),
    };
    let described = |fallback: &str| {
        pick(record, &["description"]).map_or_else(|| fallback.to_owned(), |d| truncate(d, 80))
    };
    match name {
        "Read" => file("Reading"),
        "Edit" | "MultiEdit" | "NotebookEdit" => file("Editing"),
        "Write" => file("Writing"),
        "Grep" => pick(record, &["pattern"]).map_or_else(
            || "Searching".to_owned(),
            |p| format!("Searching for {}", quoted(p)),
        ),
        "Glob" => "Finding files".into(),
        "Bash" => {
            if let Some(d) = pick(record, &["description"]) {
                return truncate(d, 80);
            }
            match pick(record, &["command"]) {
                Some(c) => format!(
                    "Running {}",
                    truncate(c.split_whitespace().next().unwrap_or(""), 40)
                ),
                None => "Running a command".into(),
            }
        }
        "WebFetch" => pick(record, &["url"]).map_or_else(
            || "Reading the web".to_owned(),
            |u| format!("Reading {}", host_of(u)),
        ),
        "WebSearch" => pick(record, &["query"]).map_or_else(
            || "Searching the web".to_owned(),
            |q| format!("Searching the web for {}", quoted(q)),
        ),
        "Task" | "Agent" => described("Delegating to a subagent"),
        "TodoWrite" => "Planning".into(),
        _ => format!("Using {}", display_name(name)),
    }
}

fn quoted(value: &str) -> String {
    format!("“{}”", truncate(value, 40))
}

fn first_sentence(text: &str) -> String {
    let line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or(text);
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let end = chars.iter().enumerate().find_map(|(i, &(at, c))| {
        let ends = matches!(c, '.' | '!' | '?');
        let next_is_space = chars.get(i + 1).is_none_or(|&(_, n)| n.is_whitespace());
        (ends && next_is_space).then_some(at + c.len_utf8())
    });
    truncate(end.map_or(line, |e| &line[..e]).trim(), 80)
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .find(|s| !s.is_empty())
        .unwrap_or(path)
}

fn host_of(url: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://")
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    {
        let authority = rest.split(['/', '?', '#', '\\']).next().unwrap_or("");
        let authority = authority.rsplit('@').next().unwrap_or(authority);
        let host = if authority.starts_with('[') {
            authority
        } else {
            authority.split(':').next().unwrap_or(authority)
        };
        if !host.is_empty() {
            return host.to_lowercase();
        }
    }
    truncate(url, 40)
}

/// `mcp__specular__create_page` reads `create page`.
fn display_name(name: &str) -> String {
    let bare = if name.starts_with("mcp__") {
        name.rsplit("__").next().unwrap_or(name)
    } else {
        name
    };
    bare.replace('_', " ")
}

fn summarize_input(name: &str, input: Option<&Value>) -> String {
    let Some(record) = input.and_then(Value::as_object) else {
        return name.to_owned();
    };
    let hint = pick(record, &["file_path", "path", "filePath"])
        .or_else(|| pick(record, &["command", "cmd"]))
        .or_else(|| pick(record, &["pattern", "query"]))
        .or_else(|| pick(record, &["url"]));
    hint.map_or_else(
        || name.to_owned(),
        |h| format!("{name} {}", truncate(h, 160)),
    )
}

fn tool_result(block: &Value) -> String {
    let is_error = block.get("is_error") == Some(&Value::Bool(true));
    let fallback = if is_error {
        "tool error"
    } else {
        "tool result"
    };
    match block.get("content") {
        Some(Value::String(content)) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return if is_error {
                    "tool error"
                } else {
                    "(empty output)"
                }
                .to_owned();
            }
            let prefix = if is_error { "tool error: " } else { "" };
            let first = trimmed.lines().next().unwrap_or("");
            truncate(&format!("{prefix}{first}"), 200)
        }
        Some(Value::Array(entries)) => {
            let parts: Vec<String> = entries.iter().filter_map(result_entry).collect();
            if parts.is_empty() {
                fallback.to_owned()
            } else {
                parts.join(" · ")
            }
        }
        _ => fallback.to_owned(),
    }
}

fn result_entry(entry: &Value) -> Option<String> {
    match entry.get("type")?.as_str()? {
        "text" => {
            let text = entry.get("text")?.as_str()?;
            (!text.trim().is_empty()).then(|| truncate(text.lines().next().unwrap_or(""), 200))
        }
        "image" => {
            let mime = entry
                .pointer("/source/media_type")
                .and_then(Value::as_str)
                .or_else(|| entry.get("mimeType").and_then(Value::as_str))
                .unwrap_or("image");
            Some(format!("image ({mime})"))
        }
        _ => None,
    }
}
