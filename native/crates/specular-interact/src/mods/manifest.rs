//! `mod.json`: our own manifest, beside `main.js`.
//!
//! `gpui-shell` has a manifest of its own (`gpui-shell.json`) that refuses
//! keys it does not know, so it cannot carry ours. A mod has no
//! `gpui-shell.json` and is loaded from `main.js`.
//!
//! The manifest is parsed here, with no file I/O, so the runtime can list
//! what is installed without the script engine and the mod host can load
//! what it lists.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use specular_doc::{CustomShape, OutlineError};

/// A version of the mod API: a generation and a revision within it.
///
/// Within a generation the surface (command names and their arguments,
/// state keys, the `specular` module's functions, `mod.json`'s keys) only
/// grows, and each growth is a new revision. A break is a new generation,
/// decided on purpose. A mod names the version it was written for, as `1`
/// (revision 0) or `"1.<revision>"`, and loads on any app of the same generation
/// whose revision is at least its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiVersion {
    /// Bumped by a break.
    pub generation: u32,
    /// Bumped by growth within a generation.
    pub revision: u32,
}

/// The mod API this build of the app speaks.
///
/// | Version | Added |
/// |---|---|
/// | 1.0 | chrome mods: keyed state, `run`, `shared`, host components, `palette` |
/// | 1.1 | mod items and tools: `items` and `tools` in `mod.json`, the `canvas/mod_items` and `tool` keys, `item.add {kind: mod}`, `item.set`, `item.draw`, `tool.preview`, `mods.install`; `commands` in `mod.json` (palette, keys, dock buttons) and the `call` key |
/// | 1.2 | the whole window (ADR 0059): layout trees and looks, `layout.switch`, `overlay.open`/`overlay.close` with `openOverlay`/`closeOverlay`, `keys.json`, `chat.*`, the `Field` component, a tool's `group`, and the `layouts`, `look`, `overlays`, `panels/sidebar`, `panels/chat`, `selection`, `canvas/items` and `canvas/info` keys |
/// | 1.3 | threads anywhere: the `chat/threads` and `chat/context` keys and a `chat/thread/<id>` key a thread, `chat.new-thread {id, open}`, `chat.send {thread}`, `chat.stop {thread}`, the `Composer` host component, the shared `mods/lib/chat.js`, and a page bridge `subscribe` to a thread's key |
/// | 1.4 | mod shapes (ADR 0060): `shapes` in `mod.json`, `shape.set`, and a shape's `custom` in `canvas/items` |
/// | 1.5 | agents on the canvas: the `agents` key and `view.follow {agent}` |
/// | 1.6 | a tool's `stroke` in `mod.json`, drawn by the app over the live stroke; `tool.stroke`; `taper` and `speed` on display-list paths; the `tool` and `call` keys published as they change |
/// | 1.7 | projects (ADR 0062): the `projects` key, a project section for each project in `panels/sidebar`, the `project.*` commands, `canvas.new {project}`, and `project` in `canvas/info` |
/// | 1.8 | sidebar sections: `section` and `starts_off` in `mod.json`, the `SidebarSections` host component, `kind` on the notes and pages sections, `title` in `panels/sidebar`, and `sidebar` and `lights` in `panels/strip` |
/// | 1.9 | sidebar menus: `menu.open {target: canvas}`, `renaming` on a canvas row of `panels/sidebar`, and `Field {control}` for a canvas's `rename` |
pub const API: ApiVersion = ApiVersion {
    generation: 1,
    revision: 9,
};

impl ApiVersion {
    /// Why a mod written for `self` cannot run on an app speaking `app`, or
    /// `None` when it can.
    pub fn refusal(self, app: Self) -> Option<String> {
        if self.generation == 0 {
            return Some("`api` starts at 1".to_owned());
        }
        if self.generation != app.generation || self.revision > app.revision {
            let update = if self > app {
                "update Specular"
            } else {
                "the mod needs updating"
            };
            return Some(format!(
                "written for mod API {self}, and this Specular speaks {app}: {update}"
            ));
        }
        None
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.generation, self.revision)
    }
}

impl std::str::FromStr for ApiVersion {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bad = || format!("`{text}` is not a mod API version such as 1 or \"1.1\"");
        let (generation, revision) = text.split_once('.').unwrap_or((text, "0"));
        Ok(Self {
            generation: generation.parse().map_err(|_| bad())?,
            revision: revision.parse().map_err(|_| bad())?,
        })
    }
}

impl Serialize for ApiVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ApiVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Generation(u32),
            Text(String),
        }
        match Written::deserialize(deserializer)? {
            Written::Generation(generation) => Ok(Self {
                generation,
                revision: 0,
            }),
            Written::Text(text) => text.parse().map_err(serde::de::Error::custom),
        }
    }
}

/// The file a mod's manifest is read from.
pub const FILE: &str = "mod.json";

/// A palette entry a mod adds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaletteEntry {
    /// What the palette shows.
    pub title: String,
    /// The command it runs.
    pub run: String,
    /// The command's arguments.
    #[serde(default)]
    pub args: Value,
}

/// What the canvas items of a mod are, when it makes any.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemSpec {
    /// The size a new item gets when `item.add` names none, `[width,
    /// height]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<[f64; 2]>,
    /// An HTML file in the mod's folder that the item shows as live web
    /// content, as a page is shown. Without it the item is the display list
    /// its mod draws with `item.draw`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web: Option<String>,
}

/// A tool a mod adds to the toolbar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolSpec {
    /// Its name within the mod. The tool is `<mod>/<name>`.
    pub name: String,
    /// What the toolbar calls it.
    pub title: String,
    /// One of the app's own glyphs, by the name `Icon` takes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// The app tool it sits under instead of a button of its own (API
    /// 1.2): `shape` lists it among the shape tool's shapes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// How the app draws the stroke while the tool is in hand (API 1.6):
    /// display-list paths with no points, drawn in order over the live
    /// stroke every frame, so the mod's script runs only on release.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stroke: Vec<specular_doc::ModDraw>,
}

impl ToolSpec {
    /// Whether it is one of the shape tool's shapes.
    pub fn is_shape(&self) -> bool {
        self.group.as_deref() == Some("shape")
    }
}

/// A shape a mod adds to the shape tool (API 1.4): a silhouette the app
/// draws, styles and themes as its own, made by the shape tool like any
/// other shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapeSpec {
    /// Its name within the mod. The shape is `<mod>/<name>`.
    pub name: String,
    /// What the shape tool's list calls it.
    pub title: String,
    /// One of the app's own glyphs, by the name `Icon` takes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Its outline as SVG path data in a 0 to 100 box (`M`, `L`, `H`, `V`,
    /// `Q`, `C`, `Z`), stretched to the shape's rect.
    pub path: String,
    /// The values a new shape starts with, which the mod reads back from
    /// `canvas/items` to make another outline with `shape.set`.
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub params: serde_json::Map<String, Value>,
}

impl ShapeSpec {
    /// A new shape of this kind, from the mod `mod_name`, or why its path is
    /// no outline.
    pub fn custom(&self, mod_name: &str) -> Result<CustomShape, OutlineError> {
        let name = format!("{mod_name}/{}", self.name);
        CustomShape::new(name, self.params.clone(), &self.path)
    }
}

/// A command a mod adds and handles itself (API 1.1): it is listed in the
/// command palette like the app's own, can have a key, and can be a button
/// in the dock while the mod's tool is in hand or its items are selected.
/// Running it publishes the `call` state key, which the mod reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModCommand {
    /// Its name within the mod. The command is `<mod>/<name>`.
    pub name: String,
    /// What the palette and the dock call it.
    pub title: String,
    /// One of the app's own glyphs, by the name `Icon` takes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Its key, such as `cmd+shift+u`, on the canvas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Whether the dock shows it as a button.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dock: bool,
    /// Whether the agent panel's header shows it as a button while a thread
    /// is open, run with `{thread}` (API 1.3).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub chat: bool,
}

/// The panel a section mod is drawn in (API 1.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SectionPlace {
    /// The sidebar, under the projects and their canvases, in name order.
    Sidebar,
}

/// A mod's `mod.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModManifest {
    /// The API version the mod was written for.
    pub api: ApiVersion,
    /// What the mod is called where a person sees it.
    pub title: String,
    /// The region it goes in when no layout names it.
    #[serde(default)]
    pub place: Option<String>,
    /// Entries it adds to the palette.
    #[serde(default)]
    pub palette: Vec<PaletteEntry>,
    /// The canvas items it makes, if any (API 1.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<ItemSpec>,
    /// The tools it adds, if any (API 1.1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolSpec>,
    /// The commands it adds and handles, if any (API 1.1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<ModCommand>,
    /// The shapes it adds to the shape tool, if any (API 1.4).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shapes: Vec<ShapeSpec>,
    /// The panel it adds a section to, whatever the layout says (API 1.8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<SectionPlace>,
    /// Whether it is off until a person turns it on in Settings > Mods
    /// (API 1.8).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub starts_off: bool,
    /// The shipped mod this one was cloned from, written by `specular mod
    /// clone`. A clone is never replaced on start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloned_from: Option<String>,
}

/// Why a mod's manifest was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError {
    /// The file.
    pub path: PathBuf,
    /// What is wrong with it.
    pub problem: String,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.problem)
    }
}

impl std::error::Error for ManifestError {}

impl ModManifest {
    /// Parses `source`, read from `path`, refusing a version this app does
    /// not speak.
    pub fn parse(source: &str, path: &Path) -> Result<Self, ManifestError> {
        let refuse = |problem: String| ManifestError {
            path: path.to_path_buf(),
            problem,
        };
        let manifest: Self = serde_json::from_str(source).map_err(|e| refuse(e.to_string()))?;
        if let Some(problem) = manifest.api.refusal(API) {
            return Err(refuse(problem));
        }
        for shape in &manifest.shapes {
            if let Err(problem) = shape.custom("") {
                return Err(refuse(format!("shape `{}`: {problem}", shape.name)));
            }
        }
        for tool in &manifest.tools {
            if (tool.stroke.iter()).any(|draw| draw.along(&[]).is_none()) {
                return Err(refuse(format!(
                    "tool `{}`: a `stroke` holds only `path` entries",
                    tool.name
                )));
            }
        }
        Ok(manifest)
    }
}
