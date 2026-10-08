use serde_json::Value;

use crate::describe;
use crate::text::truncate;

/// What a progress line is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressKind {
    /// Session start.
    System,
    /// The agent talking or thinking.
    Text,
    /// A tool call.
    ToolUse,
    /// A tool's answer.
    ToolResult,
    /// The final result.
    Result,
    /// The process's stderr.
    Stderr,
}

/// One line of the run log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// What it is about.
    pub kind: ProgressKind,
    /// The line.
    pub text: String,
    /// The present-tense status the run bar shows, when the line has one.
    pub label: Option<String>,
}

/// Something one line of the stream says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// The `claude` session id.
    Session(String),
    /// A line for the run log.
    Progress(Progress),
    /// The streaming text, replaced.
    Text(String),
    /// More streaming text.
    TextDelta(String),
    /// The run ended with this final message.
    Finished {
        /// The full final message.
        text: String,
    },
    /// The run ended in error.
    Failed {
        /// What went wrong.
        error: String,
    },
    /// The run was stopped by the caller. Never produced by [`parse_line`].
    Cancelled,
}

/// Reads one `--output-format stream-json` line. Anything unreadable or
/// unknown gives nothing.
pub fn parse_line(line: &str) -> Vec<Notice> {
    let Ok(message) = serde_json::from_str::<Value>(line.trim()) else {
        return Vec::new();
    };
    let top_level = message.get("parent_tool_use_id").is_none_or(Value::is_null);
    let content = message
        .pointer("/message/content")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    match message.get("type").and_then(Value::as_str) {
        Some("system") => system(&message),
        Some("assistant") => assistant(content, top_level),
        Some("user") => describe::user(content)
            .map(Notice::Progress)
            .into_iter()
            .collect(),
        Some("stream_event") if top_level => stream_event(&message),
        Some("result") => result(&message),
        _ => Vec::new(),
    }
}

fn system(message: &Value) -> Vec<Notice> {
    if message.get("subtype").and_then(Value::as_str) != Some("init") {
        return Vec::new();
    }
    let model = message
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("session");
    let mut out = Vec::new();
    if let Some(id) = message.get("session_id").and_then(Value::as_str) {
        out.push(Notice::Session(id.to_owned()));
    }
    let line = describe::progress(ProgressKind::System, &format!("init {model}"), None);
    out.push(Notice::Progress(line));
    out
}

fn assistant(content: &[Value], top_level: bool) -> Vec<Notice> {
    let mut out: Vec<Notice> = describe::assistant(content)
        .map(Notice::Progress)
        .into_iter()
        .collect();
    let texts: Vec<&str> = content
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    if top_level && !texts.is_empty() {
        out.push(Notice::Text(texts.join("\n\n")));
    }
    out
}

fn stream_event(message: &Value) -> Vec<Notice> {
    let event = message.get("event");
    match event.and_then(|e| e.get("type")).and_then(Value::as_str) {
        Some("message_start") => vec![Notice::Text(String::new())],
        Some("content_block_delta") => {
            let delta = event.and_then(|e| e.get("delta"));
            let is_text =
                delta.and_then(|d| d.get("type")).and_then(Value::as_str) == Some("text_delta");
            match delta.and_then(|d| d.get("text")).and_then(Value::as_str) {
                Some(text) if is_text => vec![Notice::TextDelta(text.to_owned())],
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

fn result(message: &Value) -> Vec<Notice> {
    let text = message.get("result").and_then(Value::as_str).unwrap_or("");
    let subtype = message.get("subtype").and_then(Value::as_str);
    let mut out = vec![Notice::Progress(describe::result(
        text,
        subtype.unwrap_or("done"),
    ))];
    let failed = message.get("is_error") == Some(&Value::Bool(true)) || subtype != Some("success");
    if !failed {
        out.push(Notice::Finished {
            text: text.to_owned(),
        });
        return out;
    }
    let errors: Vec<&str> = message
        .get("errors")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let error = if !errors.is_empty() {
        errors.join("; ")
    } else if !text.is_empty() {
        text.to_owned()
    } else {
        truncate(subtype.unwrap_or("error"), 200)
    };
    out.push(Notice::Failed { error });
    out
}
