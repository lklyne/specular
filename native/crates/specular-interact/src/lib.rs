//! Interaction: what the user is doing to a canvas, with no I/O.
//!
//! ```text
//! Event -> update(&mut App, Event) -> Vec<Effect>
//! ```
//!
//! [`App`] is the [`Document`](specular_doc::Document) and its
//! [`History`](specular_doc::History), plus the [`Session`]: camera,
//! selection, active [`Tool`], the [`Gesture`] in flight, hover and focus.
//! [`update`] is the only thing that changes an `App`. Whatever has to happen
//! outside it (hosting a page, forwarding input, setting the cursor) comes
//! back as an [`Effect`] for the shell to run.
//!
//! Pages are named by their [`EntityId`](specular_doc::EntityId) here. The
//! shell maps that to whatever handle its page backend hands out.
//!
//! A drag changes the document as it goes, through
//! [`Document::apply`](specular_doc::Document::apply), so everything that
//! reads the document sees the live rect. Releasing the drag records one
//! undo step; cancelling it puts the start rects back.

mod app;
mod camera;
mod comment;
mod effect;
mod event;
mod focus;
mod geometry;
mod gesture;
mod handles;
mod hit;
mod keys;
mod page_input;
mod pages;
mod placement;
mod pointer;
#[cfg(test)]
mod tests;
mod time;
mod tool;
mod update;

pub use app::{App, Focus, Selection, Session};
pub use comment::{region_annotation, region_on_canvas};
pub use effect::{Cursor, Effect};
pub use event::{Action, Event, Key, KeyInput, PageNotice, PointerInput, WheelInput};
pub use geometry::to_canvas_rect;
pub use gesture::Gesture;
pub use handles::{Corner, HANDLE_SIZE};
pub use hit::{Hit, hit_test};
pub use placement::PagePlacement;
pub use tool::Tool;
pub use update::update;
