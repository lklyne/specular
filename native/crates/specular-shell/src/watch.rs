//! The one way the shell notices a file changed on disk: its stamp, the
//! modified time and length `stat` reports, looked at every [`INTERVAL`].
//!
//! The note thread, the decode thread, the hosted pages' files and the
//! space's canvases all poll this way. [`Polled`] is the set of files one of
//! them watches; what a changed stamp means (read the text again, decode
//! again, reload a page) is theirs to say.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How often watched files are looked at for an edit from outside.
pub(crate) const INTERVAL: Duration = Duration::from_millis(500);

/// What `stat` says about a file. Two different stamps mean the file may
/// have changed; its content settles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Stamp {
    pub(crate) modified: Option<SystemTime>,
    pub(crate) len: u64,
}

/// The file's stamp, or `None` when it cannot be read: it is not there, or
/// is mid-rename by another writer.
pub(crate) fn stamp(path: &Path) -> Option<Stamp> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(Stamp {
        modified: metadata.modified().ok(),
        len: metadata.len(),
    })
}

/// A watched path and its stamp when last looked at.
type Seen = (PathBuf, Option<Stamp>);

/// Watched files by key, each key with the paths that make it up.
#[derive(Debug)]
pub(crate) struct Polled<K> {
    watched: Vec<(K, Vec<Seen>)>,
}

impl<K> Default for Polled<K> {
    fn default() -> Self {
        Self {
            watched: Vec::new(),
        }
    }
}

impl<K: PartialEq + Clone> Polled<K> {
    /// Watches `paths` under `key` from how they are now, in place of what
    /// `key` watched before.
    pub(crate) fn watch(&mut self, key: K, paths: impl IntoIterator<Item = PathBuf>) {
        let stamped = paths.into_iter().map(|path| {
            let now = stamp(&path);
            (path, now)
        });
        self.watch_stamped(key, stamped);
    }

    /// Watches `paths` under `key` as of the stamps given: ones taken
    /// before a read, so an edit during the read is seen on the next look.
    pub(crate) fn watch_stamped(&mut self, key: K, paths: impl IntoIterator<Item = Seen>) {
        self.forget(&key);
        self.watched.push((key, paths.into_iter().collect()));
    }

    /// Whether `key` is watched.
    pub(crate) fn contains(&self, key: &K) -> bool {
        self.watched.iter().any(|(watched, _)| watched == key)
    }

    /// Stops watching `key`.
    pub(crate) fn forget(&mut self, key: &K) {
        self.watched.retain(|(watched, _)| watched != key);
    }

    /// Takes `key`'s stamps again: what is on disk now is known.
    pub(crate) fn restamp(&mut self, key: &K) {
        for (watched, paths) in &mut self.watched {
            if watched == key {
                for (path, seen) in paths {
                    *seen = stamp(path);
                }
            }
        }
    }

    /// The keys with a path whose stamp moved since it was last looked at.
    /// Their stamps are taken again.
    pub(crate) fn changed(&mut self) -> Vec<K> {
        let mut changed = Vec::new();
        for (key, paths) in &mut self.watched {
            let mut moved = false;
            for (path, seen) in paths {
                let now = stamp(path);
                if now != *seen {
                    *seen = now;
                    moved = true;
                }
            }
            if moved {
                changed.push(key.clone());
            }
        }
        changed
    }

    /// Whether nothing is watched.
    pub(crate) fn is_empty(&self) -> bool {
        self.watched.is_empty()
    }
}
