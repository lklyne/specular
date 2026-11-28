//! [`ModCatalog`]: the mods installed in the app home, as the shell found
//! them.

use specular_doc::CustomShape;

use super::{ModCommand, ModManifest, ModToolId, ShapeSpec, ToolSpec};

/// One installed mod.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledMod {
    /// Its folder name.
    pub name: String,
    /// Its folder, absolute: where a web item's page is read from.
    pub dir: String,
    /// Its `mod.json`.
    pub manifest: ModManifest,
}

/// Every installed mod whose manifest reads, by name.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModCatalog {
    /// The mods, in name order.
    pub mods: Vec<InstalledMod>,
}

impl ModCatalog {
    /// The installed mod `name`.
    pub fn get(&self, name: &str) -> Option<&InstalledMod> {
        self.mods.iter().find(|installed| installed.name == name)
    }

    /// Every mod tool, in the order the toolbar lists them, each with the id
    /// [`Tool::Mod`](crate::Tool::Mod) carries.
    pub fn tools(&self) -> impl Iterator<Item = (ModToolId, &InstalledMod, &ToolSpec)> {
        (self.mods.iter())
            .flat_map(|installed| installed.manifest.tools.iter().map(move |t| (installed, t)))
            .enumerate()
            .filter_map(|(n, (installed, spec))| {
                let id = ModToolId(u16::try_from(n).ok()?);
                Some((id, installed, spec))
            })
    }

    /// The mod and the spec of the tool `id`.
    pub fn tool(&self, id: ModToolId) -> Option<(&InstalledMod, &ToolSpec)> {
        let (_, installed, spec) = self.tools().find(|(each, ..)| *each == id)?;
        Some((installed, spec))
    }

    /// Whether the tool `id` is one of the shape tool's shapes.
    pub fn is_shape(&self, id: ModToolId) -> bool {
        self.tool(id).is_some_and(|(_, spec)| spec.is_shape())
    }

    /// The tool a word such as `ink/pen` names.
    pub fn tool_named(&self, word: &str) -> Option<ModToolId> {
        let (name, tool) = word.split_once('/')?;
        let (id, ..) = (self.tools())
            .find(|(_, installed, spec)| installed.name == name && spec.name == tool)?;
        Some(id)
    }

    /// The word that names the tool `id`: `<mod>/<tool>`.
    pub fn tool_word(&self, id: ModToolId) -> Option<String> {
        let (installed, spec) = self.tool(id)?;
        Some(format!("{}/{}", installed.name, spec.name))
    }

    /// Every command a mod adds, with its mod.
    pub fn commands(&self) -> impl Iterator<Item = (&InstalledMod, &ModCommand)> {
        (self.mods.iter()).flat_map(|installed| {
            installed
                .manifest
                .commands
                .iter()
                .map(move |c| (installed, c))
        })
    }

    /// The mod command a word such as `feed/shuffle` names.
    pub fn command_named(&self, word: &str) -> Option<(&InstalledMod, &ModCommand)> {
        let (name, command) = word.split_once('/')?;
        self.commands()
            .find(|(installed, spec)| installed.name == name && spec.name == command)
    }

    /// Every shape a mod adds, with its mod, in name order.
    pub fn shapes(&self) -> impl Iterator<Item = (&InstalledMod, &ShapeSpec)> {
        (self.mods.iter()).flat_map(|installed| {
            installed
                .manifest
                .shapes
                .iter()
                .map(move |s| (installed, s))
        })
    }

    /// A new shape of the kind a word such as `star/star` names, as its
    /// mod's `mod.json` gives it.
    pub fn shape_named(&self, word: &str) -> Option<CustomShape> {
        let (name, shape) = word.split_once('/')?;
        let (installed, spec) = (self.shapes())
            .find(|(installed, spec)| installed.name == name && spec.name == shape)?;
        spec.custom(&installed.name).ok()
    }

    /// The mods that run whatever the layout says, because they make canvas
    /// items, tools or commands: each is loaded out of sight.
    pub fn background(&self) -> impl Iterator<Item = &InstalledMod> {
        self.mods.iter().filter(|installed| {
            let manifest = &installed.manifest;
            manifest.items.is_some() || !manifest.tools.is_empty() || !manifest.commands.is_empty()
        })
    }
}
