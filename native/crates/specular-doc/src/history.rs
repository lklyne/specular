//! [`History`]: undo and redo as stacks of inverse commands.

use crate::{Command, CommandError, Document};

/// The undo and redo stacks for one [`Document`].
///
/// One call to [`apply`](Self::apply) is one undo step, so a user action
/// that changes several things passes one [`Command::Batch`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct History {
    undo: Vec<Command>,
    redo: Vec<Command>,
    revision: u64,
}

impl History {
    /// An empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Runs `command` on `document` as one undo step and clears the redo
    /// stack. A refused command changes neither the document nor the history.
    pub fn apply(&mut self, document: &mut Document, command: Command) -> Result<(), CommandError> {
        let inverse = document.apply(command)?;
        self.undo.push(inverse);
        self.redo.clear();
        self.revision += 1;
        Ok(())
    }

    /// Undoes the latest step. Returns `false` when there is nothing to undo.
    ///
    /// An error means the document was changed behind the history's back
    /// (through [`Document::apply`] directly) so the step no longer fits. The
    /// step is dropped and the document is left unchanged.
    pub fn undo(&mut self, document: &mut Document) -> Result<bool, CommandError> {
        let stepped = Self::step(document, &mut self.undo, &mut self.redo)?;
        self.revision += u64::from(stepped);
        Ok(stepped)
    }

    /// Redoes the latest undone step. Returns `false` when there is nothing
    /// to redo. Errors as [`undo`](Self::undo) does.
    pub fn redo(&mut self, document: &mut Document) -> Result<bool, CommandError> {
        let stepped = Self::step(document, &mut self.redo, &mut self.undo)?;
        self.revision += u64::from(stepped);
        Ok(stepped)
    }

    /// Whether [`undo`](Self::undo) has a step to run.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether [`redo`](Self::redo) has a step to run.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// A count that goes up each time a step is applied, undone or redone,
    /// so a caller can tell that the document changed without comparing it.
    /// [`clear`](Self::clear) leaves it alone: a freshly loaded document is
    /// not a change to save.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Forgets every step, as after loading a different file.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    fn step(
        document: &mut Document,
        from: &mut Vec<Command>,
        to: &mut Vec<Command>,
    ) -> Result<bool, CommandError> {
        let Some(command) = from.pop() else {
            return Ok(false);
        };
        to.push(document.apply(command)?);
        Ok(true)
    }
}
