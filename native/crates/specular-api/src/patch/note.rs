//! A text create that is really a Document: long or structured text goes to
//! a `.md` file in the space folder, as `claimsAsNote` does in the Electron
//! app (`src/main/entities/builtin/file.ts`).

use serde_json::{Value, json};
use specular_doc::JsonMap;
use specular_interact::{Effect, note_file_name};

use super::Builder;
use crate::facts::Facts;

/// Text longer than this many UTF-16 units is a Document.
const LONG_TEXT: usize = 300;
/// What a Document's name is cut to, in characters.
const NAME_LIMIT: usize = 60;
/// The size a Document made this way has when the item gives none.
const NOTE_SIZE: f64 = 400.0;
/// What a name is called when nothing in the text names it.
const FALLBACK_NAME: &str = "Note";

/// The text of a create item that becomes a Document: no `id`, kind `text`
/// (or `note`), no `forceKind`, and text that is long, structured or
/// forced with `_forceFile`.
pub(crate) fn note_text(item: &JsonMap) -> Option<&str> {
    let kind = item.get("kind").and_then(Value::as_str)?;
    let wanted = !item.get("id").is_some_and(truthy)
        && matches!(kind, "text" | "note")
        && !item.get("forceKind").is_some_and(truthy);
    let text = item.get("text").and_then(Value::as_str)?;
    let forced = item.get("_forceFile").is_some_and(truthy);
    (wanted && (forced || is_structured(text))).then_some(text)
}

/// JavaScript's truthiness, which the CLI's flags were written against.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|number| number != 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn is_structured(text: &str) -> bool {
    text.encode_utf16().count() > LONG_TEXT
        || lines(text).any(|line| heading(line).is_some() || is_table_row(line))
        || text.contains("```")
}

/// The lines of `text`, split where a JavaScript `^` in multiline mode
/// starts one.
fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.split(['\n', '\r', '\u{2028}', '\u{2029}'])
}

/// The title of a markdown heading line: one to six `#` then whitespace.
fn heading(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches('#');
    let hashes = line.len() - rest.len();
    ((1..=6).contains(&hashes) && rest.starts_with(char::is_whitespace)).then(|| rest.trim())
}

/// A table row: a `|`, then something, then another `|`.
fn is_table_row(line: &str) -> bool {
    line.strip_prefix('|')
        .is_some_and(|rest| rest.char_indices().any(|(at, c)| c == '|' && at > 0))
}

/// The base of the file name for `text`: its first heading, else its first
/// line, with what a file name cannot hold replaced.
fn base_name(text: &str) -> String {
    let title = lines(text)
        .find_map(heading)
        .filter(|title| !title.is_empty());
    let line = title.or_else(|| lines(text).next()).unwrap_or_default();
    let safe: String = (line.chars())
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let cut: String = safe.trim().chars().take(NAME_LIMIT).collect();
    let name = cut.trim().trim_matches('.').trim();
    if name.is_empty() {
        FALLBACK_NAME.to_owned()
    } else {
        name.to_owned()
    }
}

/// The first of `base.md`, `base 2.md` and so on that nothing takes. The
/// folder's file system ignores case, so the comparison does.
fn unique_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    (1..=u32::MAX)
        .map(|number| note_file_name(base, number))
        .find(|name| !taken(&name.to_lowercase()))
        .unwrap_or_default()
}

/// A text create turned into the file create that shows its Document.
pub(super) struct NoteRoute {
    /// The item, now a `file`.
    pub(super) item: JsonMap,
}

/// Makes the Document for `item` if it is one: picks the file's name,
/// queues the write of its text, and returns the item as a file create.
pub(super) fn route(builder: &mut Builder<'_>, item: &JsonMap) -> Option<NoteRoute> {
    let text = note_text(item)?;
    let listed: Vec<String> = (builder.facts.map(Facts::space_entries))
        .unwrap_or_default()
        .iter()
        .map(|name| name.to_lowercase())
        .collect();
    let base = base_name(text);
    let file = unique_name(&base, |name| {
        listed.iter().any(|taken| taken == name)
            || builder.claimed.iter().any(|taken| taken == name)
    });
    builder.claimed.push(file.to_lowercase());
    builder.effects.push(Effect::WriteNote {
        file: file.clone(),
        text: text.to_owned(),
    });
    let mut routed = JsonMap::new();
    routed.insert("kind".to_owned(), json!("file"));
    routed.insert("file".to_owned(), json!(file));
    for key in ["canvasX", "canvasY"] {
        if let Some(value) = item.get(key) {
            routed.insert(key.to_owned(), value.clone());
        }
    }
    for key in ["width", "height"] {
        let given = item.get(key).filter(|value| value.is_number());
        routed.insert(key.to_owned(), given.cloned().unwrap_or(json!(NOTE_SIZE)));
    }
    Some(NoteRoute { item: routed })
}
