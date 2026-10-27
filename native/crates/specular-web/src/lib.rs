//! The view-and-comment web viewer (ADR 0018's web client, without live
//! editing): a published canvas drawn in a browser by the same crates that
//! draw it on the desktop.
//!
//! ```text
//! browser input --> Viewer (App + update) --> Scene --> Compositor (WebGPU or WebGL2)
//!                     |  ^
//!     uploads, new    |  |  snapshots, published files,
//!     comments        v  |  the server's comments
//!                 the web page (web/)
//! ```
//!
//! [`Viewer`] is the part that decides: what input reaches the app, what a
//! published bundle answers for a page, an image or a Document, and which
//! changes a viewer may keep. It has no GPU, so it is tested natively. The
//! `web` module (wasm32 only) owns the canvas element and the compositor and
//! is what JavaScript calls.

mod bundle;
mod keys;
mod viewer;
#[cfg(target_arch = "wasm32")]
mod web;

pub use bundle::{FORMAT, Manifest, Snapshot};
pub use keys::key_input;
pub use viewer::{Access, Mode, Upload, Viewer, ViewerError};
