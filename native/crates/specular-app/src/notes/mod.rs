//! Reading the markdown files Documents show, off the main thread, and
//! reading them again when they change on disk.
//!
//! One worker thread holds the list of watched files. It reads a file when
//! it is first watched, then looks at every file's stamp twice a second and
//! reads the ones whose stamp moved. The main thread only takes what was
//! read.

mod watch;

use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use specular_doc::Rect;

pub(crate) use self::watch::{NoteRead, ReadFailure};
use self::watch::{Watcher, Written};

/// How often the watched files are looked at for an edit from outside.
const CHECK_EVERY: Duration = Duration::from_millis(500);

enum Command {
    Watch(String),
    Unwatch(String),
    Write { file: String, text: String },
    Create { rect: Rect },
}

/// What the thread did with a write it could not carry out, or a file it
/// was asked to make.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NoteOutcome {
    /// A write was left out: the file holds `disk`, an edit from outside
    /// that had not been read yet.
    Refused {
        /// The path as the file entity names it.
        file: String,
        /// What the file holds.
        disk: String,
        /// What the write would have put there.
        ours: String,
    },
    /// A new, empty markdown file, for the Document that goes at `rect`.
    Created {
        /// The file's name in the space folder.
        file: String,
        /// Where its Document goes.
        rect: Rect,
    },
}

/// The note thread, the queue of what it has read and the queue of what
/// became of the writes and creations asked of it.
#[derive(Debug)]
pub(crate) struct NoteLoader {
    commands: Sender<Command>,
    read: Receiver<NoteRead>,
    outcomes: Receiver<NoteOutcome>,
    thread: std::thread::JoinHandle<()>,
}

impl NoteLoader {
    /// Starts the reading thread. `space` is the folder the `.canvas` file
    /// is in, if the document came from a file.
    pub(crate) fn new(space: Option<PathBuf>) -> io::Result<Self> {
        let (commands, command_queue) = mpsc::channel::<Command>();
        let (read_tx, read) = mpsc::channel();
        let (outcome_tx, outcomes) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("note-read".to_owned())
            .spawn(move || {
                let mut watcher = Watcher::new(space);
                let mut send = |read| {
                    // The loader is gone; the loop ends on the next receive.
                    let _ = read_tx.send(read);
                };
                loop {
                    match command_queue.recv_timeout(CHECK_EVERY) {
                        Ok(Command::Watch(file)) => watcher.watch(file, &mut send),
                        Ok(Command::Unwatch(file)) => watcher.unwatch(&file),
                        Ok(Command::Write { file, text }) => {
                            if let Written::Refused { disk } = watcher.write(&file, &text) {
                                let ours = text;
                                let _ = outcome_tx.send(NoteOutcome::Refused { file, disk, ours });
                            }
                        }
                        Ok(Command::Create { rect }) => match watcher.create() {
                            Ok(file) => {
                                let _ = outcome_tx.send(NoteOutcome::Created { file, rect });
                            }
                            Err(error) => tracing::warn!("no document made: {error}"),
                        },
                        Err(RecvTimeoutError::Timeout) => watcher.check(&mut send),
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            })?;
        Ok(Self {
            commands,
            read,
            outcomes,
            thread,
        })
    }

    /// Writes `text` to `file`, unless the file holds an edit from outside
    /// that has not been read yet. That comes back from
    /// [`take_outcome`](Self::take_outcome).
    pub(crate) fn write(&self, file: String, text: String) {
        self.send(Command::Write { file, text });
    }

    /// Makes an empty markdown file under a new name. The name comes back
    /// from [`take_outcome`](Self::take_outcome) with `rect`.
    pub(crate) fn create(&self, rect: Rect) {
        self.send(Command::Create { rect });
    }

    /// One refused write or made file, if there is one.
    pub(crate) fn take_outcome(&self) -> Option<NoteOutcome> {
        self.outcomes.try_recv().ok()
    }

    /// Carries out every write asked for so far, then stops the thread.
    pub(crate) fn finish(self) {
        drop(self.commands);
        if self.thread.join().is_err() {
            tracing::warn!("the note thread ended badly; a document may not be written");
        }
    }

    /// Reads `file`, as a file entity names it, and keeps watching it. Each
    /// read comes from [`take`](Self::take).
    pub(crate) fn watch(&self, file: &str) {
        self.send(Command::Watch(file.to_owned()));
    }

    /// Stops watching `file`.
    pub(crate) fn unwatch(&self, file: &str) {
        self.send(Command::Unwatch(file.to_owned()));
    }

    /// One finished read, if there is one.
    pub(crate) fn take(&self) -> Option<NoteRead> {
        self.read.try_recv().ok()
    }

    fn send(&self, command: Command) {
        if self.commands.send(command).is_err() {
            tracing::warn!("the note thread has stopped; documents will not update");
        }
    }
}
