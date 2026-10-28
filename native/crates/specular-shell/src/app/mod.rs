//! The application: one window, one compositor, one page source.
//!
//! Each loop turn the runtime takes in what the pages, the clock and the
//! loader threads have to say (`frames.rs`), and a frame is drawn only if
//! that or an input event changed something a frame shows (`demand.rs`).
//! With nothing owed the loop sleeps (`turn.rs`).

mod agent_run;
mod api_files;
mod api_run;
mod asset_run;
mod clipboard_run;
mod demand;
pub(crate) mod drop_run;
mod effects;
mod frames;
mod held_notes;
mod image_run;
mod loads;
mod lod;
mod note_run;
mod page_events;
mod repos_run;
mod runtime;
mod scripted;
mod settings;
mod share_run;
mod shots;
mod space_choice;
mod space_run;
mod title;

use glam::Vec2;
use specular_core::Camera;

pub use self::runtime::{Opening, PageOf, Runtime, RuntimeOptions, ShellWindow};
pub use self::space_choice::FolderAsk;

/// Camera a canvas with no saved one opens at, and each bench profile starts
/// from.
const START_CAMERA: Camera = Camera {
    pan: Vec2::new(40.0, 40.0),
    zoom: 0.25,
};
