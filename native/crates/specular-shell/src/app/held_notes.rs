//! The markdown files a run that does not write makes, kept in memory, so
//! a script can add and edit a Document while the disk stays untouched.

use std::collections::HashMap;

use specular_interact::note_file_name;

/// The text of each markdown file the run made, by its name.
#[derive(Debug, Default)]
pub(crate) struct HeldNotes(HashMap<String, String>);

impl HeldNotes {
    /// Makes an empty Document under the first of `Untitled Note.md`,
    /// `Untitled Note 2.md` and so on that the run has not used, as the
    /// note thread names them.
    pub(super) fn create(&mut self) -> String {
        // One more name than there are notes: at least one of them is free.
        let name = (1..=self.0.len() + 1)
            .filter_map(|count| u32::try_from(count).ok())
            .map(|count| note_file_name("Untitled Note", count))
            .find(|name| !self.0.contains_key(name))
            .unwrap_or_default();
        self.0.insert(name.clone(), String::new());
        name
    }

    /// The text of a Document the run made, or `None` for a file on disk.
    pub(super) fn get(&self, file: &str) -> Option<&str> {
        self.0.get(file).map(String::as_str)
    }

    /// Keeps a write to a Document the run made. A write to a file on disk
    /// is dropped.
    pub(super) fn write(&mut self, file: &str, text: String) {
        if let Some(held) = self.0.get_mut(file) {
            *held = text;
        }
    }
}
