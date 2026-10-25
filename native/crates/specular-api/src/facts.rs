//! What the shell knows about the disk that a write needs to be planned:
//! gathered by [`Api::answer`](crate::Api::answer) before it plans, so
//! [`Api::plan`](crate::Api::plan) itself never touches a file.

use std::collections::HashMap;

use serde_json::Value;
use specular_interact::DroppedFile;

use crate::patch::{file_path, note_text};
use crate::{Host, Method, Request};

/// What the host found at a path a request names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Found<'a> {
    /// The host was not asked.
    Unasked,
    /// There is no file there.
    Missing,
    /// The file.
    File(&'a DroppedFile),
}

/// The facts of one request.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// Each path a `file` item names, as the host found it. `None` is a
    /// path with no file. A path that is not here was not asked about, and
    /// is taken as given.
    files: HashMap<String, Option<DroppedFile>>,
    /// The names in the space folder, for naming a new Document.
    space_entries: Vec<String>,
}

impl Facts {
    /// Asks `host` about what `request` will create: the files it names
    /// and, when it makes a Document, what the space folder holds.
    pub(crate) fn gather(host: &impl Host, request: &Request) -> Self {
        let mut facts = Self::default();
        if request.method != Method::Post || request.path != "/canvas/apply" {
            return facts;
        }
        let items = request.body.get("entities").and_then(Value::as_array);
        for item in items.into_iter().flatten().filter_map(Value::as_object) {
            if let Some(path) = file_path(item) {
                facts
                    .files
                    .entry(path.to_owned())
                    .or_insert_with(|| host.inspect_file(path));
            } else if note_text(item).is_some() && facts.space_entries.is_empty() {
                facts.space_entries = host.space_entries();
            }
        }
        facts
    }

    /// What the host found at `path`.
    pub(crate) fn file(&self, path: &str) -> Found<'_> {
        match self.files.get(path) {
            None => Found::Unasked,
            Some(None) => Found::Missing,
            Some(Some(file)) => Found::File(file),
        }
    }

    /// The names in the space folder.
    pub(crate) fn space_entries(&self) -> &[String] {
        &self.space_entries
    }
}
