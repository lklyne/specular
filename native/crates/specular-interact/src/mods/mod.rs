//! Mod items and tools (ADR 0057): what the app knows of the mods that are
//! installed, what their items last drew, the tool stroke a mod tool is
//! following, and the canvas's "Install mods" ask.
//!
//! The scripts run in the shell. Everything here is plain state that
//! `update` changes and `view` reads:
//!
//! - [`ModCatalog`] is what the shell found installed
//!   ([`Event::ModCatalog`](crate::Event::ModCatalog)).
//! - A mod item's mod sends its display list with `item.draw`; it is kept
//!   here, and written into the item as its snapshot when the item's props
//!   have changed since the last one ([`live`]).
//! - A mod tool's stroke is the `tool` state key, which the mod reads and
//!   renders from; its preview comes back with `tool.preview` ([`tool`]).
//! - A canvas that uses a mod that is not installed offers to install it in
//!   the `notices` key, and its items show their snapshot until then
//!   ([`install`]).
//! - A mod's own commands join the palette, the keys and the dock, and
//!   reach the mod as the `call` key ([`commands`]).

mod action;
mod catalog;
mod commands;
mod install;
mod live;
pub mod manifest;
mod pane;
mod shape;
mod state;
mod tool;

pub use self::action::ModAction;
pub(crate) use self::action::run;
pub use self::catalog::{InstalledMod, ModCatalog};
pub use self::commands::running;
pub(crate) use self::commands::{
    bound_to, call_action, call_json, chat_buttons, running_mods, word,
};
pub(crate) use self::install::{decline, install_missing, install_notice, missing};
pub use self::live::NewModItem;
pub(crate) use self::live::{ModState, add_item, on_catalog, on_draw, set_props, turn};
pub use self::manifest::{
    API, ApiVersion, FILE, ItemSpec, ManifestError, ModCommand, ModManifest, PaletteEntry,
    ShapeSpec, ToolSpec,
};
pub use self::pane::ModRow;
pub(crate) use self::pane::mods_pane;
pub use self::shape::ShapeChange;
pub(crate) use self::shape::set_shapes;
pub(crate) use self::state::{mod_items_json, tool_json};
pub use self::tool::{ModStroke, ModToolId, StrokePhase};
pub(crate) use self::tool::{begin_stroke, cancel_stroke, drag_stroke, finish_stroke};
