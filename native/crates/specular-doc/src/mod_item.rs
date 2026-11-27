//! [`ModItem`]: a canvas item a mod defines (ADR 0057), and [`ModNeed`]: a
//! mod a canvas says it needs.
//!
//! The canvas moves, resizes, groups, syncs, undoes and saves a mod item on
//! its frame like any other. What it shows comes from its mod, and the
//! document keeps only the mod's name, the item's props and the snapshot it
//! last drew, so a reader without the mod still has something to show.

use serde::{Deserialize, Serialize};

use crate::{JsonMap, ModDraw};

/// A canvas item whose look and behaviour come from a mod.
#[derive(Debug, Clone, PartialEq)]
pub struct ModItem {
    /// The mod that draws it: the folder name under `mods/`.
    pub name: String,
    /// The item's own data, which only its mod reads. A change to it is an
    /// undo step (`SetKind`).
    pub props: JsonMap,
    /// The display list the item last drew, kept so a reader without the mod
    /// (another person, the web viewer, a sync peer) shows it as it was.
    /// Not undoable: it follows the props.
    pub snapshot: Option<Vec<ModDraw>>,
    /// What a JSON Canvas tool that knows no mods shows.
    pub fallback: ModFallback,
}

impl ModItem {
    /// A mod item of `name` with `props`, no snapshot yet, and a text
    /// fallback naming the mod.
    pub fn new(name: impl Into<String>, props: JsonMap) -> Self {
        let name = name.into();
        Self {
            fallback: ModFallback::Text(name.clone()),
            name,
            props,
            snapshot: None,
        }
    }
}

/// The JSON Canvas node a mod item is written as, for tools that do not
/// read `specular`. A mod item that wraps a URL or a file is that link or
/// file node; any other is a text node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModFallback {
    /// A `text` node with this text.
    Text(String),
    /// A `link` node to this URL.
    Link(String),
    /// A `file` node naming this path in the space folder.
    File(String),
}

/// A mod a canvas lists in its `specular.mods`: what the person is offered
/// to install when they open the canvas without it. Reading the list loads
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModNeed {
    /// The mod's folder name.
    pub name: String,
    /// The mod API version it was written for, as its `mod.json` says it.
    pub api: String,
    /// Where it can be had, relative to the space folder: the copy that
    /// travels with the canvas.
    pub from: String,
}
