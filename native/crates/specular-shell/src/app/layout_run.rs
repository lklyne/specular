//! The layouts' and themes' trip from disk (ADR 0055, ADR 0059):
//! `<app home>/layouts/` and `<app home>/themes/`, and `keys.json` beside
//! them (`keys_run.rs`), read and watched with them.
//!
//! - `layouts/<name>.json` is one whole window. The shipped ones
//!   (`default`, `classic`, `studio`) are written again on every start, so a
//!   release that changes one reaches everyone; every other file is the
//!   person's and the app never writes it. An edit applies when it is saved.
//! - `themes/<name>.json` is a theme a layout names. The shipped ones are
//!   written again on every start too.
//! - A save that is not a layout (or a theme) leaves the last good one in
//!   force and writes the reason to `<name>.error` beside it, which the next
//!   good save takes away.
//! - What a person hid and showed at run time is `layouts/.state/<name>.json`
//!   for each layout, and the layout shown last is `.state/active`, apart
//!   from the files they edit, so a click never rewrites one. Before a
//!   choice is kept, the person's own `mine.json` is shown when it is there.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use specular_interact::{
    Effect, Event, Layout, LayoutFile, LayoutsRead, SHIPPED, SHIPPED_THEMES, ThemeFile, Toggles,
};

use super::runtime::{Runtime, ShellWindow};
use crate::persist::write_atomic;
use crate::watch;

/// What a folder held when it was last read: each JSON file's name, size
/// and time of change.
type Listing = Vec<(String, u64, Option<SystemTime>)>;

/// The layouts and themes folders and what was last read from them.
#[derive(Debug)]
pub(crate) struct LayoutFiles {
    home: PathBuf,
    folder: PathBuf,
    themes: PathBuf,
    files: BTreeMap<String, LayoutFile>,
    seen: (Listing, Listing),
    checked_at: Option<Instant>,
    keys: super::keys_run::KeysFile,
}

/// The JSON files in `folder`, by name without the extension.
fn listing(folder: &Path) -> Listing {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut out: Listing = (entries.flatten())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "json") {
                return None;
            }
            let name = path.file_stem()?.to_str()?.to_owned();
            let meta = entry.metadata().ok().filter(std::fs::Metadata::is_file)?;
            Some((name, meta.len(), meta.modified().ok()))
        })
        .collect();
    out.sort();
    out
}

impl LayoutFiles {
    /// The layouts under the app's data folder `home`.
    pub(crate) fn new(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
            folder: home.join("layouts"),
            themes: home.join("themes"),
            files: BTreeMap::new(),
            seen: (Vec::new(), Vec::new()),
            checked_at: None,
            keys: super::keys_run::KeysFile::new(home),
        }
    }

    fn state_folder(&self) -> PathBuf {
        self.folder.join(".state")
    }

    fn toggles_path(&self, name: &str) -> PathBuf {
        self.state_folder().join(format!("{name}.json"))
    }

    fn active_path(&self) -> PathBuf {
        self.state_folder().join("active")
    }

    /// Writes the shipped layouts and themes over their files.
    fn write_shipped(&self) -> io::Result<()> {
        std::fs::create_dir_all(&self.folder)?;
        std::fs::create_dir_all(&self.themes)?;
        for (name, text) in SHIPPED {
            write_atomic(&self.folder.join(format!("{name}.json")), text)?;
        }
        for (name, text) in SHIPPED_THEMES {
            write_atomic(&self.themes.join(format!("{name}.json")), text)?;
        }
        Ok(())
    }

    /// Every layout file read again: the good ones by name, the errors
    /// written beside the bad ones.
    fn read_layouts(&mut self) -> BTreeMap<String, Layout> {
        let mut out = BTreeMap::new();
        for (name, _, _) in listing(&self.folder) {
            let path = self.folder.join(format!("{name}.json"));
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let file = (self.files.entry(name.clone()))
                .or_insert_with(|| LayoutFile::new(Layout::shipped()));
            let error_path = path.with_extension("error");
            if let Some(error) = file.read(&text) {
                tracing::warn!(path = %error_path.display(), "layout not read: {error}");
                warn_on(write_atomic(&error_path, &format!("{error}\n")));
                continue;
            }
            remove(&error_path);
            out.insert(name, file.layout().clone());
        }
        out
    }

    /// Every theme file that reads, by name, with the errors written
    /// beside the ones that do not.
    fn read_themes(&self) -> BTreeMap<String, ThemeFile> {
        let mut out = BTreeMap::new();
        for (name, _, _) in listing(&self.themes) {
            let path = self.themes.join(format!("{name}.json"));
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let error_path = path.with_extension("error");
            match serde_json::from_str::<ThemeFile>(&text) {
                Ok(theme) => {
                    remove(&error_path);
                    out.insert(name, theme);
                }
                Err(error) => {
                    tracing::warn!(path = %error_path.display(), "theme not read: {error}");
                    warn_on(write_atomic(&error_path, &format!("{error}\n")));
                }
            }
        }
        out
    }

    /// The toggles earlier runs kept, by layout.
    fn read_toggles(&self) -> BTreeMap<String, Toggles> {
        (listing(&self.state_folder()).into_iter())
            .filter_map(|(name, _, _)| {
                let text = std::fs::read_to_string(self.toggles_path(&name)).ok()?;
                match Toggles::parse(&text) {
                    Ok(toggles) => Some((name, toggles)),
                    Err(error) => {
                        tracing::warn!("layout toggles `{name}` not read: {error}");
                        None
                    }
                }
            })
            .collect()
    }

    /// The layout shown last, or the person's own before any was kept.
    fn read_active(&self) -> Option<String> {
        let kept = std::fs::read_to_string(self.active_path()).ok();
        kept.map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .or_else(|| {
                self.folder
                    .join("mine.json")
                    .is_file()
                    .then(|| "mine".to_owned())
            })
    }

    /// Whether either folder changed since it was last read.
    fn changed(&mut self) -> bool {
        let now = (listing(&self.folder), listing(&self.themes));
        if now == self.seen {
            return false;
        }
        self.seen = now;
        true
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
    /// The app's data folder, which holds `layouts/` and `mods/`: `None`
    /// when settings are off, as in a headless run.
    pub fn app_home(&self) -> Option<&Path> {
        self.layouts.as_ref().map(|files| files.home.as_path())
    }

    /// Writes the shipped layouts and themes, then hands the app every
    /// layout, the toggles and the choice an earlier run kept.
    pub(super) fn load_layout(&mut self) {
        let Some(files) = self.layouts.as_mut() else {
            return;
        };
        warn_on(files.write_shipped());
        files.changed();
        let read = LayoutsRead {
            layouts: files.read_layouts(),
            toggles: files.read_toggles(),
            active: files.read_active(),
            themes: files.read_themes(),
        };
        let keys = files.keys.read_if_changed();
        self.dispatch(Event::LayoutsLoaded(Box::new(read)));
        if let Some(keys) = keys {
            self.dispatch(Event::KeymapLoaded(keys));
        }
    }

    /// Reads the layouts and themes again when either folder changed on
    /// disk, at most every [`watch::INTERVAL`].
    pub(super) fn poll_layout(&mut self, now: Instant) {
        let Some(files) = self.layouts.as_mut() else {
            return;
        };
        if (files.checked_at).is_some_and(|at| now.duration_since(at) < watch::INTERVAL) {
            return;
        }
        files.checked_at = Some(now);
        if let Some(keys) = files.keys.read_if_changed() {
            self.dispatch(Event::KeymapLoaded(keys));
        }
        let Some(files) = self.layouts.as_mut() else {
            return;
        };
        if !files.changed() {
            return;
        }
        let read = LayoutsRead {
            layouts: files.read_layouts(),
            themes: files.read_themes(),
            ..LayoutsRead::default()
        };
        self.dispatch(Event::LayoutsLoaded(Box::new(read)));
    }

    /// Runs a layout effect: keeps the toggles or the choice, or resets.
    pub(super) fn layout(&mut self, effect: Effect) {
        let Some(files) = self.layouts.as_ref() else {
            return;
        };
        match effect {
            Effect::SaveLayoutToggles(name, toggles) => {
                warn_on(write_in(&files.toggles_path(&name), &toggles.to_text()));
            }
            Effect::SaveActiveLayout(name) => {
                warn_on(write_in(&files.active_path(), &format!("{name}\n")));
            }
            Effect::ResetLayout => {
                for (name, _, _) in listing(&files.state_folder()) {
                    remove(&files.toggles_path(&name));
                }
                warn_on(write_in(&files.active_path(), "default\n"));
                warn_on(files.write_shipped());
            }
            _ => {}
        }
    }
}

/// Writes `text` to `path`, making its folder.
fn write_in(path: &Path, text: &str) -> io::Result<()> {
    (path.parent().map_or(Ok(()), std::fs::create_dir_all)).and_then(|()| write_atomic(path, text))
}
