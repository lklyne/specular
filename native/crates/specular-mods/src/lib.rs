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
//!
//! Mods are trusted code (ADR 0055, "Trust"): each one gets the person's
//! files, network and processes, as a shell script would.

mod grant;
mod host;
mod manifest;
mod reads;
mod runner;
mod value;

pub use host::{MODULE, ModHost, RunError, specular_module};
pub use manifest::{API, FILE as MANIFEST_FILE, ManifestError, ModManifest, PaletteEntry};
pub use reads::Reads;
pub use runner::{Mod, ModStats};
pub use specular_interact::Revisions;
pub use value::{from_host, to_host};

/// The runtime's own types, for a host that embeds a mod's view.
pub use gpui_shell::{RuntimeMetrics, ScriptView};
