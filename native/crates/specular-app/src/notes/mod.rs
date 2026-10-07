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

use self::watch::Watcher;
pub(crate) use self::watch::{NoteRead, ReadFailure};

/// How often the watched files are looked at for an edit from outside.
const CHECK_EVERY: Duration = Duration::from_millis(500);

enum Command {
    Watch(String),
    Unwatch(String),
}

/// The reading thread and the queue of what it has read.
#[derive(Debug)]
pub(crate) struct NoteLoader {
    commands: Sender<Command>,
    read: Receiver<NoteRead>,
}

impl NoteLoader {
    /// Starts the reading thread. `space` is the folder the `.canvas` file
    /// is in, if the document came from a file.
    pub(crate) fn new(space: Option<PathBuf>) -> io::Result<Self> {
        let (commands, command_queue) = mpsc::channel::<Command>();
        let (read_tx, read) = mpsc::channel();
        std::thread::Builder::new()
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
                        Err(RecvTimeoutError::Timeout) => watcher.check(&mut send),
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            })?;
        Ok(Self { commands, read })
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
}
