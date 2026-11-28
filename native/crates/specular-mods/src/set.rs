//! [`ModSet`]: the copies of mods a window runs, loaded and kept in step.
//!
//! The window hands it every copy it runs each time it draws. It publishes
//! each copy's `props`, loads a copy the first time it is named, and loads
//! it again when its source changes: a good load takes the copy's place, a
//! bad one keeps the old build running. Either way a load that fails
//! writes `<name>.error` beside the mod and a good one takes it away, so a
//! broken save says why on disk as a broken first load does. It renders a
//! mod again only when a key it read moved.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use gpui::{AnyView, App, Window};
use serde_json::{Value, json};

use crate::host::Hosting;
use crate::runner::Mod;
use crate::shipped;

enum Slot {
    /// The build running, and when its source last changed as it was
    /// loaded or last failed to load again.
    Ready {
        loaded: Box<Mod>,
        stamp: Option<SystemTime>,
    },
    /// Why it did not load, and when its source last changed then.
    Failed {
        error: String,
        stamp: Option<SystemTime>,
    },
}

impl Slot {
    fn stamp(&self) -> Option<SystemTime> {
        match self {
            Self::Ready { stamp, .. } | Self::Failed { stamp, .. } => *stamp,
        }
    }

    fn loaded(&self) -> Option<&Mod> {
        match self {
            Self::Ready { loaded, .. } => Some(loaded),
            Self::Failed { .. } => None,
        }
    }
}

/// One copy of a mod the window runs: in a block (its id is the block's),
/// an overlay, or out of sight (its id is the mod's name).
#[derive(Debug, Clone, PartialEq)]
pub struct ModCopy {
    /// The name the copy goes by, which keys its props and its reads.
    pub id: String,
    /// The mod it is a copy of.
    pub name: String,
    /// What it is handed, as `props()`.
    pub props: Value,
}

impl ModCopy {
    /// Every copy `layout` places, in its blocks and its overlays.
    pub fn of_layout(layout: &specular_interact::Layout) -> Vec<Self> {
        (layout.mods().into_iter())
            .map(|(id, name, props)| Self {
                id,
                name: name.to_owned(),
                props: Value::Object(props.clone()),
            })
            .collect()
    }

    /// A copy of the mod `name` run out of sight, for its canvas items,
    /// tools and commands.
    pub fn background(name: &str) -> Self {
        Self {
            id: name.to_owned(),
            name: name.to_owned(),
            props: Value::Null,
        }
    }
}

/// What a block shows for the copy it names.
#[derive(Debug)]
pub enum Placed<'a> {
    /// The mod's view.
    View(AnyView),
    /// Why the mod did not load.
    Failed(&'a str),
}

/// The mods a layout names, by name.
pub struct ModSet {
    folder: PathBuf,
    hosting: Hosting,
    /// Each copy by its id, with the mod it is a copy of.
    slots: BTreeMap<String, (String, Slot)>,
    /// The state's clock when the mods were last brought in step.
    seen: Option<u64>,
    /// When the loaded mods' source was last looked at.
    looked: Option<Instant>,
}

impl std::fmt::Debug for ModSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModSet")
            .field("folder", &self.folder)
            .field("mods", &self.slots.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

/// How deep under a mod's folder its source is looked for.
const SOURCE_DEPTH: usize = 4;
/// How often the source of the loaded mods is looked at. The window draws
/// far more often; once in this long is quick enough to feel like a save
/// took.
pub const SOURCE_POLL: Duration = Duration::from_millis(200);

/// When the source of any mod in `folder` (`<app home>/mods`) last
/// changed, for a window to draw again when a mod needs loading again.
pub fn source_stamp(folder: &Path) -> Option<SystemTime> {
    newest_source(folder, SOURCE_DEPTH + 1)
}

/// When the source of the mod in `dir` last changed: its manifest and its
/// scripts, not what its runtime writes into the folder on load.
fn stamp(dir: &Path) -> Option<SystemTime> {
    newest_source(dir, SOURCE_DEPTH)
}

fn newest_source(dir: &Path, depth: usize) -> Option<SystemTime> {
    let mut newest = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "node_modules" {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let changed = if kind.is_dir() {
            (depth > 0)
                .then(|| newest_source(&entry.path(), depth - 1))
                .flatten()
        } else if is_source(&name) {
            entry.metadata().and_then(|meta| meta.modified()).ok()
        } else {
            None
        };
        newest = newest.max(changed);
    }
    newest
}

/// Whether a file in a mod's folder is source a load reads.
fn is_source(name: &str) -> bool {
    if name == crate::MANIFEST_FILE {
        return true;
    }
    !name.ends_with(".d.ts")
        && (name.rsplit_once('.'))
            .is_some_and(|(_, extension)| matches!(extension, "js" | "mjs" | "ts" | "tsx" | "jsx"))
}

impl ModSet {
    /// The mods in `folder` (`<app home>/mods`), reaching the app through
    /// `hosting`. Their own data is kept under `folder/.data/<name>`.
    pub fn new(folder: PathBuf, hosting: Hosting) -> Self {
        Self {
            folder,
            hosting,
            slots: BTreeMap::new(),
            seen: None,
            looked: None,
        }
    }

    /// What every mod shares.
    pub fn hosting(&self) -> &Hosting {
        &self.hosting
    }

    /// Brings the mods in step with `copies`, every copy the window runs
    /// now: publishes each copy's props, loads what is newly named or was
    /// fixed since it failed, and lets go of what is no longer named.
    /// Returns whether the set of copies loaded changed.
    pub fn sync(&mut self, copies: &[ModCopy], window: &mut Window, cx: &mut App) -> bool {
        for copy in copies {
            (self.hosting.revisions.borrow_mut())
                .publish(&format!("mods/{}/props", copy.id), copy.props.clone());
        }
        let before = self.slots.len();
        let reads = self.hosting.reads.clone();
        self.slots.retain(|id, (name, _)| {
            let keep = copies
                .iter()
                .any(|copy| copy.id == *id && copy.name == *name);
            if !keep {
                reads.borrow_mut().forget(id);
            }
            keep
        });
        let mut changed = self.slots.len() != before;
        let look = (self.looked).is_none_or(|at| at.elapsed() >= SOURCE_POLL);
        if look {
            self.looked = Some(Instant::now());
        }
        for copy in copies {
            let dir = self.folder.join(&copy.name);
            let known = self.slots.get(&copy.id).map(|(_, slot)| slot.stamp());
            if known.is_some() && !look {
                continue;
            }
            let now = stamp(&dir);
            if known == Some(now) {
                continue;
            }
            let old = self.slots.remove(&copy.id).map(|(_, slot)| slot);
            let slot = match (self.load(&copy.id, &copy.name, &dir, window, cx), old) {
                // A reload that fails keeps the build that runs, which
                // renders again to read what it reads afresh.
                (Err(_), Some(Slot::Ready { loaded, .. })) => {
                    self.hosting.reads.borrow_mut().forget(&copy.id);
                    self.seen = None;
                    Slot::Ready { loaded, stamp: now }
                }
                (Err(error), _) => Slot::Failed { error, stamp: now },
                (Ok(loaded), _) => {
                    changed = true;
                    Slot::Ready {
                        loaded: Box::new(loaded),
                        stamp: now,
                    }
                }
            };
            self.slots
                .insert(copy.id.clone(), (copy.name.clone(), slot));
        }
        changed
    }

    /// Loads the copy `id` of the mod `name`. A load that fails writes why
    /// beside the mod; a good one takes that away.
    fn load(
        &self,
        id: &str,
        name: &str,
        dir: &Path,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Mod, String> {
        let data = self.folder.join(".data").join(name);
        match Mod::load(id, dir, &data, &self.hosting, window, cx) {
            Ok(loaded) => {
                if let Err(error) = shipped::clear_error(dir) {
                    tracing::warn!("mod `{name}`: {error}");
                }
                Ok(loaded)
            }
            Err(error) => {
                let error = format!("{error:#}");
                tracing::warn!("mod `{name}` did not load: {error}");
                if let Err(write) = shipped::write_error(dir, &error) {
                    tracing::warn!("mod `{name}`: writing its error: {write}");
                }
                Err(error)
            }
        }
    }

    /// Renders again each mod that read a key which moved since the last
    /// call. Cheap when nothing moved, so it may be called on every draw.
    pub fn refresh(&mut self, cx: &mut App) {
        let clock = self.hosting.revisions.borrow().clock();
        if self.seen == Some(clock) {
            return;
        }
        self.seen = Some(clock);
        for (_, slot) in self.slots.values() {
            if let Some(loaded) = slot.loaded() {
                loaded.refresh_if_stale(&self.hosting, cx);
            }
        }
    }

    /// What the copy `id` shows, or `None` when no copy by that id is
    /// loaded or failing.
    pub fn placed(&self, id: &str, cx: &App) -> Option<Placed<'_>> {
        match &self.slots.get(id)?.1 {
            Slot::Ready { loaded, .. } => Some(Placed::View(loaded.view(cx))),
            Slot::Failed { error, .. } => Some(Placed::Failed(error)),
        }
    }

    /// The views of the loaded copies among `ids`, to draw out of sight so
    /// their scripts run.
    pub fn views(&self, ids: &[String], cx: &App) -> Vec<AnyView> {
        (ids.iter())
            .filter_map(|id| Some(self.slots.get(id)?.1.loaded()?.view(cx)))
            .collect()
    }

    /// The loaded copies.
    pub fn loaded(&self) -> impl Iterator<Item = &Mod> {
        self.slots.values().filter_map(|(_, slot)| slot.loaded())
    }

    /// What each loaded mod has cost, `GET /mods`'s body: renders, the mean
    /// and slowest script render in milliseconds, and the keys it read.
    pub fn stats(&self) -> Value {
        let reads = self.hosting.reads.borrow();
        let mods: Vec<Value> = (self.loaded())
            .map(|loaded| {
                let stats = loaded.stats();
                let mean = if stats.renders == 0 {
                    0.0
                } else {
                    stats.render_time.as_secs_f64() * 1000.0 / stats.renders as f64
                };
                json!({
                    "id": loaded.id(),
                    "renders": stats.renders,
                    "meanMs": (mean * 1000.0).round() / 1000.0,
                    "slowestMs": (stats.slowest.as_secs_f64() * 1e6).round() / 1000.0,
                    "reads": reads.read_by(loaded.id()),
                })
            })
            .collect();
        json!({ "mods": mods })
    }
}
