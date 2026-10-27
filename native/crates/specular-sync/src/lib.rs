//! A yrs mirror of the canvas [`Document`](specular_doc::Document), for
//! editing one canvas from several places at once.
//!
//! The [`Document`](specular_doc::Document) stays the app's only copy of the
//! canvas and every change is still a [`Command`](specular_doc::Command).
//! [`CanvasSync`] holds a yrs document beside it: local commands go out as
//! yrs updates, and a peer's update comes in as commands. Undo reverts only
//! this peer's own steps. See ADR 0048.
//!
//! The shared document has four root maps:
//!
//! - `entities`, `edges` and `annotations`: id to a map of the item's
//!   `.canvas` fields (see `fields`), plus `~z`, its fractional order key.
//! - `notes`: a markdown file's path to its text, as a Y.Text.
//!
//! No I/O happens here. The transport that carries updates between peers is
//! the caller's.

mod fields;
mod order;
mod sync;

pub use sync::{CanvasSync, SyncError};
