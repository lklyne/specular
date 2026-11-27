//! [`ModSet`]: the mods a window's layout names, loaded and kept in step.
//!
//! The window hands it the layout each time it draws. It publishes each
//! named mod's `props`, loads a mod the first time a region names it
//! (watched, so a save reloads it in place and a bad save keeps the old
//! build), writes `<name>.error` beside a mod that does not load and takes
//! it away on a good load, tries a failed mod again when its files change,
//! and renders a mod again only when a key it read moved.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use gpui::{AnyView, App, Window};
use serde_json::{Value, json};
use specular_interact::{Layout, ModPaletteEntry};

use crate::host::Hosting;
use crate::runner::Mod;
use crate::shipped;

enum Slot {
    Ready(Mod),
    /// Why it did not load, and when its files last changed then.
    Failed {
        error: String,
        stamp: Option<SystemTime>,
    },
}

/// What a region shows for the mod it names.
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
    slots: BTreeMap<String, Slot>,
    /// The state's clock when the mods were last brought in step.
    seen: Option<u64>,
}

impl std::fmt::Debug for ModSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModSet")
            .field("folder", &self.folder)
            .field("mods", &self.slots.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

/// When the files of the mod in `dir` last changed.
fn stamp(dir: &Path) -> Option<SystemTime> {
    let changed = |name: &str| {
        std::fs::metadata(dir.join(name))
            .and_then(|meta| meta.modified())
            .ok()
    };
    changed(crate::MANIFEST_FILE).max(changed("main.js"))
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
        }
    }

    /// What every mod shares.
    pub fn hosting(&self) -> &Hosting {
        &self.hosting
    }

    /// Brings the mods in step with `layout`: publishes each named mod's
    /// props, loads what is newly named or was fixed since it failed, and
    /// lets go of what is no longer named. Returns whether the set of mods
    /// loaded changed.
    pub fn sync(&mut self, layout: &Layout, window: &mut Window, cx: &mut App) -> bool {
        let mut named = Vec::new();
        for (_, node) in layout.nodes() {
            let Some(name) = node.fill.as_deref() else {
                continue;
            };
            let props = Value::Object(node.props.clone());
            (self.hosting.revisions.borrow_mut()).publish(&format!("mods/{name}/props"), props);
            if !named.iter().any(|seen| seen == name) {
                named.push(name.to_owned());
            }
        }
        let before = self.slots.len();
        let reads = self.hosting.reads.clone();
        self.slots.retain(|name, _| {
            let keep = named.contains(name);
            if !keep {
                reads.borrow_mut().forget(name);
            }
            keep
        });
        let mut changed = self.slots.len() != before;
        for name in named {
            let dir = self.folder.join(&name);
            let due = match self.slots.get(&name) {
                None => true,
                Some(Slot::Ready(_)) => false,
                Some(Slot::Failed { stamp: then, .. }) => stamp(&dir) != *then,
            };
            if due {
                let slot = self.load(&name, &dir, window, cx);
                changed |= matches!(slot, Slot::Ready(_));
                self.slots.insert(name, slot);
            }
        }
        changed
    }

    fn load(&self, name: &str, dir: &Path, window: &mut Window, cx: &mut App) -> Slot {
        let data = self.folder.join(".data").join(name);
        match Mod::load(name, dir, &data, &self.hosting, true, window, cx) {
            Ok(loaded) => {
                if let Err(error) = shipped::clear_error(dir) {
                    tracing::warn!("mod `{name}`: {error}");
                }
                Slot::Ready(loaded)
            }
            Err(error) => {
                let error = format!("{error:#}");
                tracing::warn!("mod `{name}` did not load: {error}");
                if let Err(write) = shipped::write_error(dir, &error) {
                    tracing::warn!("mod `{name}`: writing its error: {write}");
                }
                Slot::Failed {
                    error,
                    stamp: stamp(dir),
                }
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
        for slot in self.slots.values() {
            if let Slot::Ready(loaded) = slot {
                loaded.refresh_if_stale(&self.hosting, cx);
            }
        }
    }

    /// What the region filled by the mod `name` shows, or `None` when no
    /// mod by that name is loaded or failing.
    pub fn placed(&self, name: &str, cx: &App) -> Option<Placed<'_>> {
        match self.slots.get(name)? {
            Slot::Ready(loaded) => Some(Placed::View(loaded.view(cx))),
            Slot::Failed { error, .. } => Some(Placed::Failed(error)),
        }
    }

    /// The loaded mods.
    pub fn loaded(&self) -> impl Iterator<Item = &Mod> {
        self.slots.values().filter_map(|slot| match slot {
            Slot::Ready(loaded) => Some(loaded),
            Slot::Failed { .. } => None,
        })
    }

    /// The palette entries of every loaded mod's `mod.json`.
    pub fn palette(&self) -> Vec<ModPaletteEntry> {
        (self.loaded())
            .flat_map(|loaded| loaded.manifest().palette.iter())
            .map(|entry| ModPaletteEntry {
                title: entry.title.clone(),
                run: entry.run.clone(),
                args: entry.args.clone(),
            })
            .collect()
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
