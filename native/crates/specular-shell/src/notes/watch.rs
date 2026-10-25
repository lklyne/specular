//! [`Watcher`]: which markdown files are watched and what was last read
//! from each. It reads files and nothing else, so it is tested on a temp
//! folder with no thread.

use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

use specular_interact::{note_file_name, resolve_file};

use crate::persist::write_atomic;
use crate::watch::{Polled, Stamp, stamp};

/// Why a Document has no text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadFailure {
    /// There is no file at the path.
    Missing,
    /// The file could not be read, is not UTF-8, or is not on disk at all.
    Failed,
}

/// One read of a watched file: the first, or one after it changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoteRead {
    /// The path as the file entity names it.
    pub(crate) file: String,
    pub(crate) result: Result<String, ReadFailure>,
}

struct Watched {
    file: String,
    /// `None` when there is nothing on disk to read.
    path: Option<PathBuf>,
    /// What the last read reported: a hash of the text, or the failure.
    reported: Result<u64, ReadFailure>,
}

/// The watched files.
pub(super) struct Watcher {
    /// The space folder relative paths start from.
    space: Option<PathBuf>,
    watched: Vec<Watched>,
    /// The stamps of the ones on disk, by the name the file entity uses.
    stamps: Polled<String>,
}

impl Watcher {
    pub(super) fn new(space: Option<PathBuf>) -> Self {
        Self {
            space,
            watched: Vec::new(),
            stamps: Polled::default(),
        }
    }

    /// Reads `file` and reports it, then keeps it for [`check`](Self::check).
    /// A file already watched is reported again, for whoever asked anew.
    pub(super) fn watch(&mut self, file: String, report: &mut impl FnMut(NoteRead)) {
        self.unwatch(&file);
        let path = resolve_file(&file, self.space.as_deref());
        if path.is_none() {
            tracing::debug!(file, "document is not a file on disk");
        }
        let (stamp, result) = read(path.as_deref());
        let reported = result
            .as_ref()
            .map(|text| hash(text))
            .map_err(|&failure| failure);
        report(NoteRead {
            file: file.clone(),
            result,
        });
        if let Some(path) = &path {
            (self.stamps).watch_stamped(file.clone(), [(path.clone(), stamp)]);
        }
        self.watched.push(Watched {
            file,
            path,
            reported,
        });
    }

    pub(super) fn unwatch(&mut self, file: &str) {
        self.watched.retain(|watched| watched.file != file);
        self.stamps.forget(&file.to_owned());
    }

    /// Reads every file whose stamp moved since its last read, and reports
    /// the ones that now read differently. A `touch`, or a save that wrote
    /// the same text, reports nothing.
    pub(super) fn check(&mut self, report: &mut impl FnMut(NoteRead)) {
        for file in self.stamps.changed() {
            let Some(watched) = self.watched.iter_mut().find(|it| it.file == file) else {
                continue;
            };
            let (_, result) = read(watched.path.as_deref());
            let reported = result
                .as_ref()
                .map(|text| hash(text))
                .map_err(|&failure| failure);
            if reported != watched.reported {
                watched.reported = reported;
                report(NoteRead {
                    file: watched.file.clone(),
                    result,
                });
            }
        }
    }
}

/// What became of a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Written {
    /// The file holds the text.
    Done,
    /// The file holds `disk`, which nobody here has seen, so it was left
    /// alone.
    Refused {
        /// What the file holds.
        disk: String,
    },
    /// The file could not be written.
    Failed,
}

/// The name a new Document's file takes, before `.md` and a number.
const NEW_NOTE_NAME: &str = "Untitled Note";

impl Watcher {
    /// Writes `text` to `file` unless the file holds a text that was never
    /// reported from here or written from here: an edit from outside that
    /// the app has not heard of yet. That text is then what is known of the
    /// file, so the app can decide and write again.
    pub(super) fn write(&mut self, file: &str, text: &str) -> Written {
        let Some(path) = resolve_file(file, self.space.as_deref()) else {
            tracing::warn!(file, "document is not a file on disk; not written");
            return Written::Failed;
        };
        let known = (self.watched.iter()).find(|watched| watched.file == file);
        let known = known.map(|watched| watched.reported);
        let (_, disk) = read(Some(&path));
        let mut seen = |reported: Result<u64, ReadFailure>| {
            if let Some(watched) = self.watched.iter_mut().find(|it| it.file == file) {
                watched.reported = reported;
                self.stamps.restamp(&file.to_owned());
            }
        };
        match disk {
            Ok(disk) if disk == text => {
                seen(Ok(hash(text)));
                Written::Done
            }
            Ok(disk) if known != Some(Ok(hash(&disk))) => {
                seen(Ok(hash(&disk)));
                Written::Refused { disk }
            }
            Ok(_) | Err(_) => match write_atomic(&path, text) {
                Ok(()) => {
                    seen(Ok(hash(text)));
                    Written::Done
                }
                Err(error) => {
                    tracing::warn!(path = %path.display(), "document not written: {error}");
                    Written::Failed
                }
            },
        }
    }

    /// Makes an empty markdown file in the space folder under the first of
    /// `Untitled Note.md`, `Untitled Note 2.md` and so on that no file has,
    /// and returns its name.
    pub(super) fn create(&self) -> io::Result<String> {
        let space = (self.space.as_deref())
            .ok_or_else(|| io::Error::other("the canvas has no folder to put a document in"))?;
        for number in 1..=u32::MAX {
            let name = note_file_name(NEW_NOTE_NAME, number);
            let made = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(space.join(&name));
            match made {
                Ok(_) => return Ok(name),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::other("every document name is taken"))
    }
}

/// The file's text, with the stamp taken just before the read: an edit that
/// lands during the read moves the stamp again and is read on the next check.
fn read(path: Option<&Path>) -> (Option<Stamp>, Result<String, ReadFailure>) {
    let Some(path) = path else {
        return (None, Err(ReadFailure::Failed));
    };
    let stamp = stamp(path);
    let result = std::fs::read_to_string(path).map_err(|error| {
        tracing::debug!(path = %path.display(), "document cannot be read: {error}");
        if error.kind() == io::ErrorKind::NotFound {
            ReadFailure::Missing
        } else {
            ReadFailure::Failed
        }
    });
    (stamp, result)
}

fn hash(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh space folder under the system temp dir, removed on drop.
    struct Space(PathBuf);

    impl Space {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("specular-watch-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("notes")).unwrap();
            Self(dir)
        }
    }

    impl Drop for Space {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_write_is_refused_over_a_text_nobody_has_read() {
        let space = Space::new("refuse");
        let path = space.0.join("a.md");
        std::fs::write(&path, "one").unwrap();
        let mut watcher = Watcher::new(Some(space.0.clone()));
        watcher.watch("a.md".to_owned(), &mut |_| {});
        assert_eq!(watcher.write("a.md", "ours"), Written::Done);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "ours");

        std::fs::write(&path, "theirs, from outside").unwrap();
        let refused = Written::Refused {
            disk: "theirs, from outside".to_owned(),
        };
        assert_eq!(watcher.write("a.md", "ours again"), refused);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "theirs, from outside"
        );
        // The refusal told the app, which may now write over it. Neither
        // that text nor our own write is reported a second time.
        assert_eq!(watcher.write("a.md", "ours again"), Written::Done);
        let mut reads = Vec::new();
        watcher.check(&mut |read| reads.push(read));
        assert_eq!(reads, []);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "ours again");
    }
}
