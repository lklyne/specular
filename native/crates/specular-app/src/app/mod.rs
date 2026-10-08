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
mod drop_run;
mod effects;
#[cfg(target_os = "macos")]
mod file_menu;
mod frames;
mod gpu_window;
mod image_run;
mod input;
mod lod;
#[cfg(target_os = "macos")]
mod menu_bar;
mod note_run;
mod page_events;
mod repos_run;
mod runtime;
mod scripted;
mod settings;
mod shots;
mod space_choice;
mod space_run;
mod title;
mod turn;
mod winit_window;

use glam::Vec2;
use specular_core::Camera;

pub(crate) use self::api_run::ShellEvent;
pub use self::runtime::{Opening, PageOf, Runtime, RuntimeOptions, ShellWindow};
pub(crate) use self::winit_window::Shell;
pub use self::winit_window::run_window;

/// Camera a canvas with no saved one opens at, and each bench profile starts
/// from.
const START_CAMERA: Camera = Camera {
    pan: Vec2::new(40.0, 40.0),
    zoom: 0.25,
};
