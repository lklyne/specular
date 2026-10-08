//! [`History`]: undo and redo as stacks of inverse commands.

use crate::{Command, CommandError, Document};

/// The undo and redo stacks for one [`Document`].
///
/// One call to [`apply`](Self::apply) is one undo step, so a user action
/// that changes several things passes one [`Command::Batch`].
///
/// A step also carries the caller's view state `S` from either side of it,
/// which for the app is the selection. Undoing a step hands back the state
/// from before it and redoing hands back the state from after, so what was
/// selected travels with the change and no call site has to put it back.
/// The document knows nothing about `S`; with `S = ()` a history is the two
/// stacks and nothing else.
#[derive(Debug, Clone, PartialEq)]
pub struct History<S = ()> {
    undo: Vec<Step<S>>,
    redo: Vec<Step<S>>,
    revision: u64,
    /// The latest step's `after` has not been given yet.
    open: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Step<S> {
    /// What takes the document across this step, in the direction of the
    /// stack it is on.
    command: Command,
    before: S,
    after: S,
}

impl<S> Default for History<S> {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            revision: 0,
            open: false,
        }
    }
}

impl History {
    /// An empty history that carries no state with its steps.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S: Clone> History<S> {
    /// Runs `command` on `document` as one undo step and clears the redo
    /// stack. A refused command changes neither the document nor the history.
    pub fn apply(&mut self, document: &mut Document, command: Command) -> Result<(), CommandError>
    where
        S: Default,
    {
        self.apply_from(document, command, S::default())
    }

    /// [`apply`](Self::apply), with `before` as the state an undo of this
    /// step returns to. The state a redo returns to is `before` as well
    /// until [`settle`](Self::settle) gives it.
    pub fn apply_from(
        &mut self,
        document: &mut Document,
        command: Command,
        before: S,
    ) -> Result<(), CommandError> {
        let command = document.apply(command)?;
        self.undo.push(Step {
            command,
            after: before.clone(),
            before,
        });
        self.redo.clear();
        self.revision += 1;
        self.open = true;
        Ok(())
    }

    /// Whether the latest step is still waiting for [`settle`](Self::settle).
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Gives the latest step the state it left behind, once whatever made
    /// the step has finished with that state. Does nothing when the latest
    /// step already has one, so it is safe to call after every event.
    pub fn settle(&mut self, after: S) {
        if self.open
            && let Some(step) = self.undo.last_mut()
        {
            step.after = after;
        }
        self.open = false;
    }

    /// Undoes the latest step and returns the state from before it, or
    /// `None` when there is nothing to undo.
    ///
    /// An error means the document was changed behind the history's back
    /// (through [`Document::apply`] directly) so the step no longer fits. The
    /// step is dropped and the document is left unchanged.
    pub fn undo(&mut self, document: &mut Document) -> Result<Option<S>, CommandError> {
        self.open = false;
        let stepped = Self::step(document, &mut self.undo, &mut self.redo)?;
        self.revision += u64::from(stepped.is_some());
        Ok(stepped.map(|step| step.before.clone()))
    }

    /// Redoes the latest undone step and returns the state from after it, or
    /// `None` when there is nothing to redo. Errors as [`undo`](Self::undo)
    /// does.
    pub fn redo(&mut self, document: &mut Document) -> Result<Option<S>, CommandError> {
        self.open = false;
        let stepped = Self::step(document, &mut self.redo, &mut self.undo)?;
        self.revision += u64::from(stepped.is_some());
        Ok(stepped.map(|step| step.after.clone()))
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
        self.open = false;
    }

    /// Moves the top step of `from` across the document onto `to`, and
    /// returns it as it now sits there.
    fn step<'a>(
        document: &mut Document,
        from: &mut Vec<Step<S>>,
        to: &'a mut Vec<Step<S>>,
    ) -> Result<Option<&'a Step<S>>, CommandError> {
        let Some(step) = from.pop() else {
            return Ok(None);
        };
        let command = document.apply(step.command)?;
        to.push(Step { command, ..step });
        Ok(to.last())
    }
}
