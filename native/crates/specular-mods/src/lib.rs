//! The mod host (ADR 0055).
//!
//! A mod is a folder holding `mod.json` and `main.js`. `gpui-shell` runs the
//! script and turns what it describes into GPUI elements; this crate gives it
//! the one module that reaches the app, `specular`, and keeps the rest of the
//! app free of the runtime:
//!
//! - [`ModHost`] is what the app offers a mod: keyed state to read, commands
//!   to run by name, values shared between mods. The shell implements it over
//!   the command registry and the keyed state in `specular-interact`.
//! - [`Reads`] records which keys each mod read while it rendered, so a mod is
//!   rendered again only when one of those keys moved.
//! - [`ModManifest`] is our own `mod.json`, with the API generation a mod was
//!   written for.
//! - [`Mod`] is one loaded mod: its runtime, its view and its manifest.
//! - [`Components`] are the Rust-built elements a mod places but cannot draw
//!   itself: `Icon`, and the window's `Zoom` and `ToolOptions`.
//! - [`ModSet`] is the mods a window's layout names, loaded, kept in step
//!   and rendered again only when a key one read moved.
//! - [`shipped`] is the mods the app ships, with the guide and
//!   `specular.d.ts`, written into the app's mods folder on every start.
//! - [`check`] loads a mod against a state fixture and renders it once, for
//!   `specular mod check`.
//!
//! Mods are trusted code (ADR 0055, "Trust"): each one gets the person's
//! files, network and processes, as a shell script would.

mod check;
mod chrome;
mod components;
mod grant;
mod host;
mod manifest;
mod reads;
mod runner;
mod set;
pub mod shipped;
mod value;

pub use check::{CheckReport, check, fresh_state};
pub use chrome::chrome_colors;
pub use components::{Build, Components, HOST_COMPONENTS};
pub use host::{Hosting, MODULE, ModHost, RunError, specular_module};
pub use manifest::read_manifest;
pub use reads::Reads;
pub use runner::{Mod, ModStats};
pub use set::{ModSet, Placed};
pub use specular_interact::Revisions;
pub use specular_interact::mods::{
    API, ApiVersion, FILE as MANIFEST_FILE, ItemSpec, ManifestError, ModManifest, PaletteEntry,
    ToolSpec,
};
pub use value::{from_host, to_host};

/// The runtime's own types, for a host that embeds a mod's view.
pub use gpui_shell::{ComponentArgs, HostValue, RuntimeMetrics, ScriptView, with_current_app};
