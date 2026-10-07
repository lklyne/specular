//! The editor's own undo stack, which lives only as long as the edit
//! session. The document sees none of it: ending the session is one
//! document step.

/// The text and selection at one moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) text: String,
    pub(crate) caret: usize,
    pub(crate) anchor: usize,
}

/// What kind of change was made, which decides whether it joins the step
/// before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Change {
    /// Characters typed one after another are one step.
    Typing,
    /// So are characters deleted one after another.
    Deleting,
    /// Anything else (a paste, a cut, a line break, a composition) is a
    /// step of its own.
    Single,
}

/// A run of typing or deleting still open to more of the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Run {
    change: Change,
    /// Where the run left the caret. The next change joins only from here.
    caret: usize,
}

/// Undo and redo for one edit session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct EditHistory {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    run: Option<Run>,
}

impl EditHistory {
    /// Records that the text was `before` and a `change` then moved the
    /// caret to `caret`.
    pub(crate) fn record(&mut self, before: Snapshot, change: Change, caret: usize) {
        let joins = change != Change::Single
            && self.run
                == Some(Run {
                    change,
                    caret: before.caret,
                })
            && before.caret == before.anchor;
        if !joins {
            self.undo.push(before);
        }
        self.redo.clear();
        self.run = match change {
            Change::Typing | Change::Deleting => Some(Run { change, caret }),
            Change::Single => None,
        };
    }

    /// Ends the open run, so the next change starts a new step. Moving the
    /// caret does this.
    pub(crate) fn seal(&mut self) {
        self.run = None;
    }

    /// Takes back the latest step if it turned out to change nothing: a
    /// composition that was cancelled.
    pub(crate) fn forget_if_unchanged(&mut self, text: &str) {
        if self.undo.last().is_some_and(|last| last.text == text) {
            self.undo.pop();
        }
    }

    /// The state to go back to, given the state now.
    pub(crate) fn undo(&mut self, now: Snapshot) -> Option<Snapshot> {
        self.run = None;
        let back = self.undo.pop()?;
        self.redo.push(now);
        Some(back)
    }

    /// The state to go forward to, given the state now.
    pub(crate) fn redo(&mut self, now: Snapshot) -> Option<Snapshot> {
        self.run = None;
        let forward = self.redo.pop()?;
        self.undo.push(now);
        Some(forward)
    }
}
