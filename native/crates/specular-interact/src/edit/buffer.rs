//! [`TextEdit`]: the text being edited, its caret and selection, and the
//! changes everything else is built from.

use std::ops::Range;

use specular_doc::{EntityId, Rect};

use super::history::{Change, EditHistory, Snapshot};
use super::note::NoteSave;
use super::segment;

/// What kind of text is being edited, which decides the keys that apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    /// A text or sticky entity: Enter continues a bullet list and Tab nests
    /// it.
    Text,
    /// A shape's label.
    Label,
    /// A Document: the source of a markdown file. It has the list keys, the
    /// whole formatting set, and it scrolls instead of growing.
    Note,
}

/// What the session started from, for ending it as one document step.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Origin {
    pub(crate) text: String,
    pub(crate) rect: Rect,
    /// Whether the entity was placed for this session. It is in the document
    /// with no undo step until the session ends.
    pub(crate) created: bool,
}

/// An edit session: one entity's text, as it stands with the edits so far.
/// The document keeps the old text until the session ends.
///
/// Offsets are bytes into [`text`](Self::text), on grapheme boundaries.
#[derive(Debug, Clone, PartialEq)]
pub struct TextEdit {
    pub(crate) entity: EntityId,
    pub(crate) target: Target,
    pub(crate) text: String,
    pub(crate) caret: usize,
    pub(crate) anchor: usize,
    pub(crate) composition: Option<Range<usize>>,
    /// Where up and down are aiming, in the layout's x. A run of them keeps
    /// to one column across short lines.
    pub(crate) preferred_x: Option<f32>,
    pub(crate) history: EditHistory,
    pub(crate) origin: Origin,
    /// When the caret or the text last changed, on the session's clock. The
    /// caret's blink restarts from here, so it stays solid while typing.
    pub(crate) active_ms: u64,
    /// The file the text is written to as it changes, for a Document.
    pub(crate) note: Option<NoteSave>,
}

impl TextEdit {
    /// A session on `entity` with all of `text` selected, as an editor that
    /// has just taken focus has it.
    pub(crate) fn new(entity: EntityId, target: Target, text: &str, origin: Origin) -> Self {
        Self {
            entity,
            target,
            text: text.to_owned(),
            caret: text.len(),
            anchor: 0,
            composition: None,
            preferred_x: None,
            history: EditHistory::default(),
            origin,
            active_ms: 0,
            note: None,
        }
    }

    /// The entity whose text this is.
    pub fn entity(&self) -> &EntityId {
        &self.entity
    }

    /// Whether this is an edit of a Document's markdown source.
    pub fn is_note(&self) -> bool {
        self.target == Target::Note
    }

    /// The text as edited so far, with any composition in it.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Where the caret is.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Where the selection started. Equal to the caret when nothing is
    /// selected.
    pub fn anchor(&self) -> usize {
        self.anchor
    }

    /// The selected bytes, empty when nothing is selected.
    pub fn selection(&self) -> Range<usize> {
        self.caret.min(self.anchor)..self.caret.max(self.anchor)
    }

    /// The text the input method is still composing, which is drawn
    /// underlined.
    pub fn composition(&self) -> Option<Range<usize>> {
        self.composition.clone()
    }

    /// The selected text.
    pub(crate) fn selected(&self) -> &str {
        &self.text[self.selection()]
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            caret: self.caret,
            anchor: self.anchor,
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.text = snapshot.text;
        self.caret = snapshot.caret;
        self.anchor = snapshot.anchor;
        self.preferred_x = None;
    }

    /// Puts the caret at `offset`. `extend` keeps the anchor, so the
    /// selection grows or shrinks to it.
    pub(crate) fn move_to(&mut self, offset: usize, extend: bool) {
        self.caret = segment::floor(&self.text, offset);
        if !extend {
            self.anchor = self.caret;
        }
        self.preferred_x = None;
        self.history.seal();
    }

    /// Selects `range`, with the caret at its end.
    pub(crate) fn select(&mut self, range: Range<usize>) {
        self.anchor = segment::floor(&self.text, range.start);
        self.caret = segment::floor(&self.text, range.end);
        self.preferred_x = None;
        self.history.seal();
    }

    /// Replaces `range` with `with` and leaves the caret after it. No undo
    /// step: [`change`](Self::change) wraps this.
    pub(crate) fn splice(&mut self, range: Range<usize>, with: &str) {
        self.text.replace_range(range.clone(), with);
        self.caret = range.start + with.len();
        self.anchor = self.caret;
        self.preferred_x = None;
    }

    /// Replaces `range` with `with`, keeping the caret and anchor on the
    /// text they were on.
    pub(crate) fn splice_around(&mut self, range: Range<usize>, with: &str) {
        let shift = |offset: usize| {
            if offset >= range.end {
                offset - range.len() + with.len()
            } else {
                offset.min(range.start)
            }
        };
        (self.caret, self.anchor) = (shift(self.caret), shift(self.anchor));
        self.text.replace_range(range, with);
        self.preferred_x = None;
    }

    /// Runs `edit` as one step of the editor's undo, if it changes the text.
    /// Returns whether it did.
    pub(crate) fn change(&mut self, change: Change, edit: impl FnOnce(&mut Self)) -> bool {
        let before = self.snapshot();
        edit(self);
        if self.text == before.text {
            return false;
        }
        self.history.record(before, change, self.caret);
        true
    }

    /// Types `text` over the selection.
    pub(crate) fn insert(&mut self, text: &str, change: Change) -> bool {
        self.change(change, |edit| edit.splice(edit.selection(), text))
    }

    /// Deletes `range`.
    pub(crate) fn delete(&mut self, range: Range<usize>, change: Change) -> bool {
        self.change(change, |edit| edit.splice(range, ""))
    }

    /// Records the state as a step before a change made outside
    /// [`change`](Self::change): the start of a composition.
    pub(crate) fn checkpoint(&mut self) {
        let before = self.snapshot();
        self.history.record(before, Change::Single, self.caret);
    }

    /// Steps the editor's undo back. Returns whether there was a step.
    pub(crate) fn undo(&mut self) -> bool {
        let now = self.snapshot();
        match self.history.undo(now) {
            Some(back) => {
                self.restore(back);
                true
            }
            None => false,
        }
    }

    /// Steps the editor's undo forward. Returns whether there was a step.
    pub(crate) fn redo(&mut self) -> bool {
        let now = self.snapshot();
        match self.history.redo(now) {
            Some(forward) => {
                self.restore(forward);
                true
            }
            None => false,
        }
    }
}
