//! What a headless run keeps in memory in place of the system clipboard
//! and of the markdown files a session makes, so a script can copy, paste
//! and add a Document while the disk and the real clipboard stay untouched.

use std::collections::HashMap;

/// The clipboard and the Documents made during the run.
#[derive(Debug, Default)]
pub(super) struct StandIns {
    /// What was last copied, by the app or by a `clipboard` step.
    pub(super) clipboard: Option<String>,
    /// The text of each markdown file the run made, by its name.
    notes: HashMap<String, String>,
}

impl StandIns {
    /// Makes an empty Document under the first of `Untitled Note.md`,
    /// `Untitled Note 2.md` and so on that the run has not used, as the
    /// shell's note thread names them.
    pub(super) fn create_note(&mut self) -> String {
        // One more name than there are notes: at least one of them is free.
        let name = (1..=self.notes.len() + 1)
            .map(|count| match count {
                1 => "Untitled Note.md".to_owned(),
                count => format!("Untitled Note {count}.md"),
            })
            .find(|name| !self.notes.contains_key(name))
            .unwrap_or_default();
        self.notes.insert(name.clone(), String::new());
        name
    }

    /// The text of a Document the run made, or `None` for a file on disk.
    pub(super) fn note(&self, file: &str) -> Option<&str> {
        self.notes.get(file).map(String::as_str)
    }

    /// Keeps a write to a Document the run made. A write to a file on disk
    /// is dropped.
    pub(super) fn write_note(&mut self, file: &str, text: String) {
        if let Some(held) = self.notes.get_mut(file) {
            *held = text;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_documents_take_the_next_free_name() {
        let mut stand_ins = StandIns::default();
        assert_eq!(stand_ins.create_note(), "Untitled Note.md");
        assert_eq!(stand_ins.create_note(), "Untitled Note 2.md");
    }

    #[test]
    fn only_documents_the_run_made_are_written() {
        let mut stand_ins = StandIns::default();
        let file = stand_ins.create_note();
        stand_ins.write_note(&file, "# Plan".to_owned());
        stand_ins.write_note("on-disk.md", "lost".to_owned());
        assert_eq!(stand_ins.note(&file), Some("# Plan"));
        assert_eq!(stand_ins.note("on-disk.md"), None);
    }
}
