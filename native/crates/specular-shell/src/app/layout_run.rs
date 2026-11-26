//! The layout file's trip from disk (ADR 0055): `<app home>/layouts/`.
//!
//! - `default.json` is the shipped layout, written again on every start,
//!   so a release that changes it reaches everyone. An edit to it applies
//!   when it is saved.
//! - A save that is not a layout leaves the last good one in force and
//!   writes the reason to `default.error` beside it, which the next good
//!   save takes away.
//! - What a person hid and showed at run time is `.state/default.json`,
//!   apart from the file they edit, so a click never rewrites it.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use specular_interact::{Effect, Event, Layout, LayoutFile, Toggles};

use super::runtime::{Runtime, ShellWindow};
use crate::persist::write_atomic;
use crate::watch::{self, Polled};

/// The layouts folder and what was last read from it.
#[derive(Debug)]
pub(crate) struct LayoutFiles {
    folder: PathBuf,
    file: LayoutFile,
    polled: Polled<()>,
    checked_at: Option<Instant>,
}

impl LayoutFiles {
    /// The layouts under the app's data folder `home`.
    pub(crate) fn new(home: &Path) -> Self {
        Self {
            folder: home.join("layouts"),
            file: LayoutFile::default(),
            polled: Polled::default(),
            checked_at: None,
        }
    }

    fn layout_path(&self) -> PathBuf {
        self.folder.join("default.json")
    }

    fn error_path(&self) -> PathBuf {
        self.folder.join("default.error")
    }

    fn toggles_path(&self) -> PathBuf {
        self.folder.join(".state").join("default.json")
    }

    /// Writes the shipped layout over `default.json` and watches it.
    fn write_shipped(&mut self) -> io::Result<()> {
        std::fs::create_dir_all(&self.folder)?;
        let path = self.layout_path();
        write_atomic(&path, &Layout::shipped().to_text())?;
        self.file.reset();
        remove(&self.error_path());
        self.polled.watch((), [path]);
        Ok(())
    }

    /// The layout file read again, when it changed: the layout to hand the
    /// app, or `None` when there is nothing new or it did not read.
    fn reread(&mut self) -> Option<Layout> {
        let text = std::fs::read_to_string(self.layout_path()).ok()?;
        let error_path = self.error_path();
        if let Some(error) = self.file.read(&text) {
            tracing::warn!(path = %error_path.display(), "layout not read: {error}");
            warn_on(write_atomic(&error_path, &format!("{error}\n")));
            return None;
        }
        remove(&error_path);
        Some(self.file.layout().clone())
    }

    /// The toggles an earlier run kept, if any read.
    fn read_toggles(&self) -> Option<Toggles> {
        let text = std::fs::read_to_string(self.toggles_path()).ok()?;
        match Toggles::parse(&text) {
            Ok(toggles) => Some(toggles),
            Err(error) => {
                tracing::warn!("layout toggles not read: {error}");
                None
            }
        }
    }
}

fn remove(path: &Path) {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => {
            tracing::warn!(path = %path.display(), "not removed: {error}");
        }
        Ok(()) | Err(_) => {}
    }
}

fn warn_on(result: io::Result<()>) {
    if let Err(error) = result {
        tracing::warn!("layout file not written: {error}");
    }
}

impl<W: ShellWindow> Runtime<W> {
    /// Writes the shipped layout, then hands the app it and the toggles an
    /// earlier run kept.
    pub(super) fn load_layout(&mut self) {
        let Some(files) = self.layouts.as_mut() else {
            return;
        };
        warn_on(files.write_shipped());
        let layout = files.reread();
        let toggles = files.read_toggles();
        if let Some(layout) = layout {
            self.dispatch(Event::LayoutLoaded(Box::new(layout)));
        }
        if let Some(toggles) = toggles {
            self.dispatch(Event::LayoutToggles(toggles));
        }
    }

    /// Reads the layout file again when it changed on disk, at most every
    /// [`watch::INTERVAL`].
    pub(super) fn poll_layout(&mut self, now: Instant) {
        let Some(files) = self.layouts.as_mut() else {
            return;
        };
        if (files.checked_at).is_some_and(|at| now.duration_since(at) < watch::INTERVAL) {
            return;
        }
        files.checked_at = Some(now);
        if files.polled.changed().is_empty() {
            return;
        }
        if let Some(layout) = files.reread() {
            self.dispatch(Event::LayoutLoaded(Box::new(layout)));
        }
    }

    /// Runs a layout effect: keeps the toggles, or resets the files.
    pub(super) fn layout(&mut self, effect: Effect) {
        match effect {
            Effect::SaveLayoutToggles(toggles) => self.save_layout_toggles(&toggles),
            Effect::ResetLayout => self.reset_layout(),
            _ => {}
        }
    }

    /// Keeps what was toggled, apart from the layout file.
    fn save_layout_toggles(&self, toggles: &Toggles) {
        let Some(files) = self.layouts.as_ref() else {
            return;
        };
        let path = files.toggles_path();
        let written = (path.parent().map_or(Ok(()), std::fs::create_dir_all))
            .and_then(|()| write_atomic(&path, &toggles.to_text()));
        warn_on(written);
    }

    /// Sets the toggles aside and writes the shipped layout again.
    fn reset_layout(&mut self) {
        let Some(files) = self.layouts.as_mut() else {
            return;
        };
        remove(&files.toggles_path());
        warn_on(files.write_shipped());
    }
}
