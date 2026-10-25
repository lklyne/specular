//! [`FileSync`]: when to save and what to do when the file changes under
//! us. No I/O here; [`Persistence`](super::Persistence) does what this says.

use std::time::SystemTime;

/// How long after the latest change the save runs. Electron's autosave
/// waits the same.
pub(crate) const SAVE_DEBOUNCE_MS: u64 = 350;
/// The longest a change waits to be saved while newer changes keep
/// restarting the debounce.
const SAVE_MAX_WAIT_MS: u64 = 2_000;
/// How often the file is looked at for an edit from outside.
const DISK_CHECK_MS: u64 = 500;
/// How long a failed save waits before it is tried again.
const SAVE_RETRY_MS: u64 = 5_000;

/// What `stat` says about the file. Two different stamps mean the file may
/// have changed; the text settles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stamp {
    pub(crate) modified: Option<SystemTime>,
    pub(crate) len: u64,
}

/// What a loop turn should do about the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// Nothing.
    Idle,
    /// Write the document.
    Save,
    /// Stat the file and report it with [`FileSync::disk_stamp`].
    CheckDisk,
}

/// What to do with text read from a file whose stamp moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiskChange {
    /// It is what we last read or wrote. Nothing changed.
    Unchanged,
    /// Someone else edited it and we have nothing unsaved: load it.
    Reload,
    /// Someone else edited it while we have unsaved changes: ours win, and
    /// the pending save will overwrite theirs.
    KeepOurs,
}

/// The save debounce and the record of what is on disk. Times are
/// milliseconds on any clock that does not go backwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSync {
    /// When the pending save runs.
    save_due: Option<u64>,
    /// When the oldest change still waiting to be saved was made.
    unsaved_since: Option<u64>,
    /// When the file is next looked at.
    check_due: u64,
    /// The file's text as we last read or wrote it.
    synced: String,
    /// The file's stamp when `synced` was taken.
    stamp: Option<Stamp>,
}

impl FileSync {
    /// A file just read as `text`, with nothing to save.
    pub(crate) fn new(text: String, stamp: Option<Stamp>) -> Self {
        Self {
            save_due: None,
            unsaved_since: None,
            check_due: 0,
            synced: text,
            stamp,
        }
    }

    /// The document changed. The save runs once changes have stopped for
    /// [`SAVE_DEBOUNCE_MS`], or [`SAVE_MAX_WAIT_MS`] after the first of a
    /// run of changes that does not stop.
    pub(crate) fn request_save(&mut self, now_ms: u64) {
        let since = *self.unsaved_since.get_or_insert(now_ms);
        self.save_due = Some((now_ms + SAVE_DEBOUNCE_MS).min(since + SAVE_MAX_WAIT_MS));
    }

    /// Whether a change is waiting to be written.
    pub(crate) fn has_unsaved(&self) -> bool {
        self.save_due.is_some()
    }

    /// What to do this turn. While `dragging`, nothing: the document holds
    /// the drag's unfinished rects, which a cancel would take back, and a
    /// reload would pull the document out from under the drag.
    pub(crate) fn step(&mut self, now_ms: u64, dragging: bool) -> Step {
        if dragging {
            return Step::Idle;
        }
        if self.save_due.is_some_and(|due| due <= now_ms) {
            self.save_due = None;
            self.unsaved_since = None;
            return Step::Save;
        }
        if self.check_due <= now_ms {
            self.check_due = now_ms + DISK_CHECK_MS;
            return Step::CheckDisk;
        }
        Step::Idle
    }

    /// Takes the pending save, if any, to run now: the app is closing.
    pub(crate) fn take_unsaved(&mut self) -> bool {
        self.unsaved_since = None;
        self.save_due.take().is_some()
    }

    /// We wrote `text` and the file now has `stamp`.
    pub(crate) fn saved(&mut self, text: String, stamp: Option<Stamp>) {
        self.synced = text;
        self.stamp = stamp;
    }

    /// The file was moved and has `stamp` where it is now. Its text is
    /// what it was.
    pub(crate) fn restamp(&mut self, stamp: Option<Stamp>) {
        self.stamp = stamp;
    }

    /// The save could not be written. It is tried again later, unless a
    /// newer change is already waiting.
    pub(crate) fn save_failed(&mut self, now_ms: u64) {
        self.save_due.get_or_insert(now_ms + SAVE_RETRY_MS);
    }

    /// The file's stamp now. Returns whether the file has to be read: its
    /// stamp moved since we last read or wrote it.
    pub(crate) fn disk_stamp(&self, stamp: Stamp) -> bool {
        self.stamp != Some(stamp)
    }

    /// The text read from a file whose stamp moved to `stamp`.
    pub(crate) fn disk_text(&mut self, text: &str, stamp: Stamp) -> DiskChange {
        self.stamp = Some(stamp);
        if text == self.synced {
            return DiskChange::Unchanged;
        }
        if self.has_unsaved() {
            return DiskChange::KeepOurs;
        }
        text.clone_into(&mut self.synced);
        DiskChange::Reload
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::*;

    fn stamp(seconds: u64, len: u64) -> Stamp {
        Stamp {
            modified: Some(UNIX_EPOCH + Duration::from_secs(seconds)),
            len,
        }
    }

    fn synced() -> FileSync {
        let mut sync = FileSync::new("on disk".to_owned(), Some(stamp(1, 7)));
        // The first turn looks at the file; get that out of the way.
        assert_eq!(sync.step(0, false), Step::CheckDisk);
        sync
    }

    #[test]
    fn a_save_runs_once_350_ms_after_the_change() {
        let mut sync = synced();
        sync.request_save(100);
        assert!(sync.has_unsaved());
        assert_eq!(sync.step(449, false), Step::Idle);
        assert_eq!(sync.step(450, false), Step::Save);
        assert!(!sync.has_unsaved());
        assert_ne!(sync.step(451, false), Step::Save);
    }

    #[test]
    fn nothing_is_saved_or_checked_while_dragging() {
        let mut sync = synced();
        sync.request_save(100);
        assert_eq!(sync.step(10_000, true), Step::Idle);
        assert_eq!(sync.step(10_001, false), Step::Save);
    }

    #[test]
    fn an_external_edit_with_unsaved_changes_keeps_ours_and_says_so_once() {
        let mut sync = synced();
        sync.request_save(100);
        assert_eq!(sync.disk_text("theirs", stamp(2, 6)), DiskChange::KeepOurs);
        assert!(
            !sync.disk_stamp(stamp(2, 6)),
            "the same edit is not read again"
        );
        assert!(sync.has_unsaved(), "and our save still runs");
    }
}
