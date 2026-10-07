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

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    /// A fresh space folder under the system temp dir, removed on drop.
    struct Space(PathBuf);

    impl Space {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("specular-notes-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Space {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn wait(loader: &NoteLoader) -> NoteRead {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(read) = loader.take() {
                return read;
            }
            assert!(Instant::now() < deadline, "the note thread never answered");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn a_watched_file_is_read_then_read_again_when_it_changes() {
        let space = Space::new("thread");
        std::fs::write(space.0.join("plan.md"), "# one").unwrap();
        let loader = NoteLoader::new(Some(space.0.clone())).unwrap();
        loader.watch("plan.md");
        let first = wait(&loader);
        assert_eq!(
            (first.file.as_str(), first.result),
            ("plan.md", Ok("# one".to_owned()))
        );
        // Longer, so the stamp moves even on a coarse clock.
        std::fs::write(space.0.join("plan.md"), "# one and two").unwrap();
        assert_eq!(wait(&loader).result, Ok("# one and two".to_owned()));
    }

    #[test]
    fn a_write_lands_before_the_thread_is_finished() {
        let space = Space::new("finish");
        let loader = NoteLoader::new(Some(space.0.clone())).unwrap();
        loader.create(Rect::new(0.0, 0.0, 300.0, 300.0));
        let deadline = Instant::now() + Duration::from_secs(10);
        let made = loop {
            if let Some(outcome) = loader.take_outcome() {
                break outcome;
            }
            assert!(Instant::now() < deadline, "the note thread never answered");
            std::thread::sleep(Duration::from_millis(2));
        };
        let NoteOutcome::Created { file, .. } = made else {
            panic!("a file was asked for: {made:?}");
        };
        assert_eq!(file, "Untitled Note.md");
        // As the app does: watch the new file, then write what is typed.
        loader.watch(&file);
        loader.write(file, "typed".to_owned());
        loader.finish();
        let written = std::fs::read_to_string(space.0.join("Untitled Note.md")).unwrap();
        assert_eq!(written, "typed");
    }
}
