//! `<app home>/keys.json` from disk (ADR 0059): the person's own keys. The
//! app never writes it. A save that reads replaces the keys in force; one
//! that does not leaves them and writes the reason to `keys.error` beside
//! it, which the next good save takes away. Taking the file away goes back
//! to the table's keys alone.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use specular_interact::Keymap;

use crate::persist::write_atomic;

/// The file and when it was last read.
#[derive(Debug)]
pub(crate) struct KeysFile {
    path: PathBuf,
    seen: Option<(u64, Option<SystemTime>)>,
}

impl KeysFile {
    /// `keys.json` in the app's data folder `home`.
    pub(crate) fn new(home: &Path) -> Self {
        Self {
            path: home.join("keys.json"),
            seen: None,
        }
    }

    /// The keys, when the file changed since it was last read and reads.
    /// A file gone is no keys of the person's.
    pub(crate) fn read_if_changed(&mut self) -> Option<Keymap> {
        let meta = std::fs::metadata(&self.path).ok();
        let now = meta.as_ref().map(|meta| (meta.len(), meta.modified().ok()));
        if now == self.seen {
            return None;
        }
        let was_there = self.seen.is_some();
        self.seen = now;
        let error_path = self.path.with_extension("error");
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return was_there.then(Keymap::default);
        };
        match Keymap::parse(&text) {
            Ok(keymap) => {
                if error_path.exists()
                    && let Err(error) = std::fs::remove_file(&error_path)
                {
                    tracing::warn!(path = %error_path.display(), "not removed: {error}");
                }
                Some(keymap)
            }
            Err(error) => {
                tracing::warn!(path = %self.path.display(), "keys not read: {error}");
                if let Err(error) = write_atomic(&error_path, &format!("{error}\n")) {
                    tracing::warn!("keys.error not written: {error}");
                }
                None
            }
        }
    }
}
