//! [`Watcher`]: which markdown files are watched and what was last read
//! from each. It reads files and nothing else, so it is tested on a temp
//! folder with no thread.

use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

use crate::images::resolve::resolve;
use crate::persist::{Stamp, stamp};

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
    /// The file's stamp at the last read; `None` when it was not there.
    stamp: Option<Stamp>,
    /// What the last read reported: a hash of the text, or the failure.
    reported: Result<u64, ReadFailure>,
}

/// The watched files.
pub(super) struct Watcher {
    /// The space folder relative paths start from.
    space: Option<PathBuf>,
    watched: Vec<Watched>,
}

impl Watcher {
    pub(super) fn new(space: Option<PathBuf>) -> Self {
        Self {
            space,
            watched: Vec::new(),
        }
    }

    /// Reads `file` and reports it, then keeps it for [`check`](Self::check).
    /// A file already watched is reported again, for whoever asked anew.
    pub(super) fn watch(&mut self, file: String, report: &mut impl FnMut(NoteRead)) {
        self.unwatch(&file);
        let path = resolve(&file, self.space.as_deref());
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
        self.watched.push(Watched {
            file,
            path,
            stamp,
            reported,
        });
    }

    pub(super) fn unwatch(&mut self, file: &str) {
        self.watched.retain(|watched| watched.file != file);
    }

    /// Reads every file whose stamp moved since its last read, and reports
    /// the ones that now read differently. A `touch`, or a save that wrote
    /// the same text, reports nothing.
    pub(super) fn check(&mut self, report: &mut impl FnMut(NoteRead)) {
        for watched in &mut self.watched {
            let Some(path) = watched.path.as_deref() else {
                continue;
            };
            if stamp(path) == watched.stamp {
                continue;
            }
            let (stamp, result) = read(Some(path));
            let reported = result
                .as_ref()
                .map(|text| hash(text))
                .map_err(|&failure| failure);
            watched.stamp = stamp;
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

        fn write(&self, file: &str, text: &str) {
            std::fs::write(self.0.join(file), text).unwrap();
        }

        fn watcher(&self) -> Watcher {
            Watcher::new(Some(self.0.clone()))
        }
    }

    impl Drop for Space {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn watch(watcher: &mut Watcher, file: &str) -> Vec<NoteRead> {
        let mut reads = Vec::new();
        watcher.watch(file.to_owned(), &mut |read| reads.push(read));
        reads
    }

    fn check(watcher: &mut Watcher) -> Vec<(String, Result<String, ReadFailure>)> {
        let mut reads = Vec::new();
        watcher.check(&mut |read: NoteRead| reads.push((read.file, read.result)));
        reads
    }

    fn text(file: &str, text: &str) -> (String, Result<String, ReadFailure>) {
        (file.to_owned(), Ok(text.to_owned()))
    }

    #[test]
    fn watching_reads_the_file_from_the_space_folder() {
        let space = Space::new("reads");
        space.write("notes/plan.md", "# Plan\n");
        let reads = watch(&mut space.watcher(), "notes/plan.md");
        assert_eq!(reads.len(), 1);
        assert_eq!(reads[0].file, "notes/plan.md");
        assert_eq!(reads[0].result.as_deref(), Ok("# Plan\n"));
    }

    #[test]
    fn a_changed_file_is_read_again_and_an_unchanged_one_is_not() {
        let space = Space::new("changes");
        space.write("a.md", "one");
        space.write("b.md", "same");
        let mut watcher = space.watcher();
        watch(&mut watcher, "a.md");
        watch(&mut watcher, "b.md");
        assert_eq!(check(&mut watcher), []);
        // A different length, so the stamp moves even on a coarse clock.
        space.write("a.md", "one, two");
        assert_eq!(check(&mut watcher), [text("a.md", "one, two")]);
        assert_eq!(check(&mut watcher), []);
    }

    #[test]
    fn a_missing_file_is_reported_once_and_read_when_it_appears() {
        let space = Space::new("appears");
        let mut watcher = space.watcher();
        let reads = watch(&mut watcher, "later.md");
        assert_eq!(reads[0].result, Err(ReadFailure::Missing));
        assert_eq!(check(&mut watcher), []);
        space.write("later.md", "here now");
        assert_eq!(check(&mut watcher), [text("later.md", "here now")]);
        std::fs::remove_file(space.0.join("later.md")).unwrap();
        assert_eq!(
            check(&mut watcher),
            [("later.md".to_owned(), Err(ReadFailure::Missing))]
        );
    }

    #[test]
    fn a_file_that_is_not_text_and_a_web_url_fail() {
        let space = Space::new("fails");
        std::fs::write(space.0.join("bad.md"), [0xff, 0xfe, 0x00]).unwrap();
        let mut watcher = space.watcher();
        assert_eq!(
            watch(&mut watcher, "bad.md")[0].result,
            Err(ReadFailure::Failed)
        );
        let url = "https://example.com/readme.md";
        assert_eq!(watch(&mut watcher, url)[0].result, Err(ReadFailure::Failed));
        assert_eq!(check(&mut watcher), []);
    }

    #[test]
    fn an_unwatched_file_is_left_alone() {
        let space = Space::new("unwatch");
        space.write("a.md", "one");
        let mut watcher = space.watcher();
        watch(&mut watcher, "a.md");
        watcher.unwatch("a.md");
        space.write("a.md", "one, two");
        assert_eq!(check(&mut watcher), []);
    }

    #[test]
    fn watching_again_reports_the_file_again() {
        let space = Space::new("again");
        space.write("a.md", "one");
        let mut watcher = space.watcher();
        watch(&mut watcher, "a.md");
        assert_eq!(watch(&mut watcher, "a.md")[0].result.as_deref(), Ok("one"));
        space.write("a.md", "one, two");
        assert_eq!(check(&mut watcher), [text("a.md", "one, two")]);
    }
}
