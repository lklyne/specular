//! A headless driver for feature tests: build an app, script input into it,
//! assert on the document, the session and the effects.
//!
//! ```
//! use specular_testkit::TestApp;
//!
//! let mut app = TestApp::with_pages(2);
//! app.drag((200.0, 150.0), (260.0, 130.0));
//! assert_eq!(app.rect("p1").x, 160.0);
//! app.assert_undo_returns_to_start();
//! ```
//!
//! [`TestApp`] wraps an [`App`](specular_interact::App) and sends every
//! input through [`update`](specular_interact::update), the same function
//! the shell calls, so a test exercises the real routing. Input methods
//! return `&mut TestApp` and chain. The effects each `update` returns pile up
//! until [`TestApp::take_effects`] drains them.
//!
//! This crate is a dev-dependency everywhere but the shell, whose headless
//! `--snapshot` and `--script` runs drive a [`TestApp`]. A crate's own unit
//! tests cannot use it on that crate's types (the testkit links the crate's library build,
//! whose types differ from the test build's), so tests that use it live
//! under `tests/`.
#![expect(
    clippy::panic,
    clippy::missing_panics_doc,
    reason = "every helper panics to fail the test it is called from"
)]

mod app;
mod fixtures;
mod input;
mod measure;
mod scene_snapshot;
mod snapshot;

pub use app::TestApp;
pub use fixtures::{
    connected, document, drawing, file, group, inside, labelled, note, page, pages, plain_text,
    shape, sticky, text, with_edge,
};
pub use input::{ALT, CMD, CMD_SHIFT, CTRL, SHIFT};
pub use measure::FixedAdvance;
pub use scene_snapshot::scene_snapshot;
pub use snapshot::doc_snapshot;

#[doc(hidden)]
pub use insta;
