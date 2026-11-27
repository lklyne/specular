//! [`ModState`]: what the app holds for mods between events, and the
//! changes `update` makes to mod items.

use std::collections::{BTreeSet, HashMap};

use glam::DVec2;
use serde_json::Value;
use specular_doc::{Command, Entity, EntityId, ItemId, JsonMap, Kind, ModDraw, ModItem, Rect};

use super::{ModCatalog, ModStroke};
use crate::{App, Effect, Tool, live, pages};

/// The size a mod item gets when neither the command nor the mod's
/// `items.size` names one.
const DEFAULT_SIZE: [f64; 2] = [320.0, 240.0];

/// Mod state that is not in the document.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ModState {
    /// What is installed and turned on.
    pub(crate) catalog: ModCatalog,
    /// What is installed, turned on or not.
    pub(crate) installed: ModCatalog,
    /// The display list each mod item's mod last sent, by item.
    pub(crate) draws: HashMap<EntityId, Vec<ModDraw>>,
    /// The props and size each item had when its snapshot was last
    /// written, this run.
    snapshot_props: HashMap<EntityId, (JsonMap, [f64; 2])>,
    /// The armed mod tool's preview, in canvas units.
    pub(crate) preview: Vec<ModDraw>,
    /// The armed mod tool's stroke, kept after the release until the next
    /// press so the mod sees `up` and commits.
    pub(crate) stroke: Option<ModStroke>,
    /// The canvases whose "Install mods" ask was declined, by canvas id.
    pub(crate) declined: BTreeSet<String>,
    /// The last mod command run, the `call` key.
    pub(crate) call: Option<super::commands::ModCall>,
}

/// The shell listed what is installed. A mod tool that is no longer there
/// is put down, and the page hosts follow the web items that are now (or no
/// longer) shown live.
pub(crate) fn on_catalog(app: &mut App, catalog: ModCatalog, effects: &mut Vec<Effect>) {
    app.mods.installed = catalog;
    turn(app, effects);
}

/// Brings the mods that run in step with what is installed and the mods
/// the settings turn off: their tools, commands and items go, and the
/// layouts draw the app's own panels where they were.
pub(crate) fn turn(app: &mut App, effects: &mut Vec<Effect>) {
    let off = &app.settings.mods_off;
    let catalog = ModCatalog {
        mods: (app.mods.installed.mods.iter())
            .filter(|installed| !off.contains(&installed.name))
            .cloned()
            .collect(),
    };
    app.chrome.set_off(off);
    if catalog == app.mods.catalog {
        return;
    }
    let before = pages::snapshot(app);
    let tool = if let Tool::Mod(id) = app.session.tool {
        app.mods.catalog.tool_word(id)
    } else {
        None
    };
    app.mods.catalog = catalog;
    if let Some(word) = tool {
        app.session.tool = app
            .mods
            .catalog
            .tool_named(&word)
            .map_or(Tool::Select, Tool::Mod);
    }
    pages::reconcile(&before, app, effects);
}

/// The mod of item `id` sent what it draws. The item's snapshot is written
/// from it when its props or size changed since the last snapshot this
/// run, or it has none: so the file follows the item, not every redraw. Returns
/// whether the snapshot changed, which is a change to save.
pub(crate) fn on_draw(app: &mut App, id: EntityId, draws: Vec<ModDraw>) -> bool {
    let Some(Entity {
        kind: Kind::Mod(item),
        rect,
        ..
    }) = app.document.entity(&id)
    else {
        return false;
    };
    let drawn = (item.props.clone(), [rect.width, rect.height]);
    let due = item.snapshot.is_none() || app.mods.snapshot_props.get(&id) != Some(&drawn);
    let mut written = false;
    if due {
        written = app.document.set_mod_snapshot(&id, draws.clone());
        app.mods.snapshot_props.insert(id.clone(), drawn);
    }
    app.mods.draws.insert(id, draws);
    written
}

/// The command that sets the keys of `patch` on mod item `id`'s props, a
/// `null` taking its key away, or `None` when `id` is no mod item or nothing
/// would change.
pub(crate) fn set_props(app: &App, id: &EntityId, patch: &JsonMap) -> Option<Command> {
    let Kind::Mod(item) = &app.document.entity(id)?.kind else {
        return None;
    };
    let mut next = item.clone();
    for (key, value) in patch {
        match value {
            Value::Null => next.props.shift_remove(key),
            value => next.props.insert(key.clone(), value.clone()),
        };
    }
    (next.props != item.props).then(|| Command::SetKind {
        id: id.clone(),
        kind: Box::new(Kind::Mod(next)),
    })
}

/// What `item.add {kind: mod}` places.
#[derive(Debug, Clone, PartialEq)]
pub struct NewModItem {
    /// The mod.
    pub name: String,
    /// The item's props.
    pub props: JsonMap,
    /// Its size, or the mod's own.
    pub size: Option<DVec2>,
}

/// Places a mod item with its top-left at `at`, in front, as one undo step,
/// and selects it. The canvas lists the mod it needs, and the shell copies
/// the mod into the space folder so the canvas carries it. Nothing happens
/// for a mod that is not installed.
pub(crate) fn add_item(
    app: &mut App,
    new: NewModItem,
    at: Option<DVec2>,
    effects: &mut Vec<Effect>,
) {
    let Some(installed) = app.mods.catalog.get(&new.name) else {
        return;
    };
    let size = (new.size.map(|size| [size.x, size.y]))
        .or_else(|| installed.manifest.items.as_ref()?.size)
        .unwrap_or(DEFAULT_SIZE);
    let api = installed.manifest.api.to_string();
    let id = EntityId::from(app.fresh_id().as_str());
    let at = at
        .unwrap_or_else(|| crate::add_item::view_middle(app) - DVec2::new(size[0], size[1]) / 2.0);
    let rect = Rect::new(at.x, at.y, size[0], size[1]);
    let item = ModItem {
        api,
        ..ModItem::new(&new.name, new.props)
    };
    // The first of its mod's items on this canvas brings a copy of the mod
    // into the space, which the canvas then lists.
    if !(app.document.mod_needs().iter()).any(|need| need.name == new.name) {
        effects.push(Effect::BundleMod(new.name));
    }
    let entity = Entity::new(id.clone(), rect, Kind::Mod(item));
    app.mods.preview.clear();
    live::create(app, entity, effects);
    app.session.selection.set([ItemId::Entity(id)]);
}

impl App {
    /// What the mod item `id` shows: the display list its mod last sent,
    /// else the snapshot it was saved with. `None` until either exists.
    pub fn mod_draws(&self, id: &EntityId) -> Option<&[ModDraw]> {
        if let Some(sent) = self.mods.draws.get(id) {
            return Some(sent);
        }
        let Kind::Mod(item) = &self.document.entity(id)?.kind else {
            return None;
        };
        item.snapshot.as_deref()
    }

    /// The armed mod tool's preview, in canvas units.
    pub fn mod_preview(&self) -> &[ModDraw] {
        &self.mods.preview
    }

    /// What is installed.
    pub fn mod_catalog(&self) -> &ModCatalog {
        &self.mods.catalog
    }

    /// Every mod installed, turned on or not.
    pub fn installed_mods(&self) -> &ModCatalog {
        &self.mods.installed
    }
}
