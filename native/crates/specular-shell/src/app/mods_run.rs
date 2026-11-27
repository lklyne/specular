//! Mods on disk for their items and tools (ADR 0057): what is installed in
//! the mods folder, installing a canvas's mods from its space folder, and
//! copying a mod into the space folder so the canvas carries it.
//!
//! The mods folder is `<app home>/mods`, which the window also loads mods
//! from. A headless run has none until a script names one (`mods FOLDER`).

use std::io;
use std::path::{Path, PathBuf};

use specular_doc::ModNeed;
use specular_interact::Event;
use specular_interact::mods::{FILE, InstalledMod, ModCatalog, ModManifest};

use super::runtime::{Runtime, ShellWindow};

/// Files a mod's load writes beside it, which are not the mod.
const WRITTEN_ON_LOAD: [&str; 2] = ["jsconfig.json", "gpui-kit.d.ts"];

/// What is installed in `folder`: each folder whose `mod.json` reads, in
/// name order. One that does not read is left out and logged.
pub(crate) fn scan(folder: &Path) -> ModCatalog {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return ModCatalog::default();
    };
    let mut mods: Vec<InstalledMod> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join(FILE).is_file())
        .filter_map(|entry| {
            let dir = entry.path();
            let name = entry.file_name().to_str()?.to_owned();
            let path = dir.join(FILE);
            let text = std::fs::read_to_string(&path).ok()?;
            match ModManifest::parse(&text, &path) {
                Ok(manifest) => Some(InstalledMod {
                    name,
                    dir: dir.to_string_lossy().into_owned(),
                    manifest,
                }),
                Err(error) => {
                    tracing::warn!("mod `{name}` is not listed: {error}");
                    None
                }
            }
        })
        .collect();
    mods.sort_by(|a, b| a.name.cmp(&b.name));
    ModCatalog { mods }
}

/// Copies the mod folder `from` over `to`, leaving out what a load writes
/// and the error a load left.
pub(crate) fn copy_mod(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        let left_out = WRITTEN_ON_LOAD.iter().any(|written| name == *written)
            || Path::new(&name)
                .extension()
                .is_some_and(|ext| ext == "error");
        if left_out {
            continue;
        }
        let (source, target) = (entry.path(), to.join(&name));
        if source.is_dir() {
            copy_mod(&source, &target)?;
        } else {
            std::fs::copy(&source, &target)?;
        }
    }
    Ok(())
}

/// Whether `name` can be a mod's folder: one plain path segment.
fn plain(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\'])
}

impl<W: ShellWindow> Runtime<W> {
    /// The folder mods are installed in, if this run has one.
    pub fn mods_folder(&self) -> Option<&Path> {
        self.mods_folder.as_deref()
    }

    /// Uses `folder` as the mods folder and tells the app what is in it.
    pub fn use_mods_folder(&mut self, folder: PathBuf) {
        self.mods_folder = Some(folder);
        self.rescan_mods();
    }

    /// Tells the app what is installed now.
    pub fn rescan_mods(&mut self) {
        let catalog = self
            .mods_folder
            .as_deref()
            .map_or_else(Default::default, scan);
        self.dispatch(Event::ModCatalog(catalog));
    }

    /// Installs each of `needs` from the space folder into the mods folder,
    /// then tells the app what is installed. The person asked for it.
    pub(super) fn install_mods(&mut self, needs: &[ModNeed]) {
        let (Some(space), Some(folder)) = (self.space_folder(), self.mods_folder()) else {
            tracing::warn!("mods not installed: no space folder or no mods folder");
            return;
        };
        let (space, folder) = (space.to_path_buf(), folder.to_path_buf());
        for need in needs.iter().filter(|need| plain(&need.name)) {
            let from = space.join(&need.from);
            let inside = !need.from.split(['/', '\\']).any(|part| part == "..");
            if !inside || !from.join(FILE).is_file() {
                tracing::warn!("mod `{}` not installed: no mod at {}", need.name, need.from);
                continue;
            }
            match copy_mod(&from, &folder.join(&need.name)) {
                Ok(()) => tracing::info!("installed mod `{}` from {}", need.name, from.display()),
                Err(error) => tracing::warn!("mod `{}` not installed: {error}", need.name),
            }
        }
        self.rescan_mods();
    }

    /// Copies the installed mod `name` into the space folder's `mods/`.
    pub(super) fn bundle_mod(&self, name: &str) {
        let (Some(space), Some(folder)) = (self.space_folder(), self.mods_folder()) else {
            return;
        };
        if !plain(name) {
            return;
        }
        let to = space.join("mods").join(name);
        if let Err(error) = copy_mod(&folder.join(name), &to) {
            tracing::warn!("mod `{name}` not copied into the space: {error}");
        }
    }
}
