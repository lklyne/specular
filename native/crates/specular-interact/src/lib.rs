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
//! undo step; cancelling it puts everything back as the drag found it. A
//! gesture that creates an entity works the same way: the entity is in the
//! document while it is dragged out, and [`App::creating`] names it.

mod anchor;
mod app;
mod bindings;
mod camera;
mod caps;
mod clone;
mod comment;
mod cursor;
mod draw;
mod edge_path;
mod effect;
mod event;
mod focus;
mod geometry;
mod gesture;
mod grid;
mod handles;
mod hit;
mod images;
mod live;
mod marquee;
mod move_drag;
mod notes;
mod page_input;
mod pages;
mod place;
mod placement;
mod pointer;
mod resize;
mod resize_drag;
mod scope;
mod select;
mod strokes;
mod time;
mod tool;
mod tool_defaults;
mod update;
mod verbs;

pub use anchor::{anchors_to_pages, page_anchor_for};
pub use app::{App, Focus, Selection, Session};
pub use bindings::{BINDINGS, Binding, Chord, Context, binding_for};
pub use caps::{AspectMode, aspect_mode, has_anchors, min_size};
pub use comment::{region_annotation, region_on_canvas};
pub use draw::DrawStroke;
pub use edge_path::EdgeCurve;
pub use effect::{Cursor, Effect};
pub use event::{Action, Event, Key, KeyInput, PageNotice, PointerInput, WheelInput};
pub use geometry::to_canvas_rect;
pub use gesture::Gesture;
pub use handles::{Corner, HANDLE_SIZE, Handle, HandleOwner, OUTLINE_PADDING};
pub use hit::{Hit, hit_test};
pub use images::{Image, ImageKey, ImageNotice, ImageState, is_image_file};
pub use marquee::MarqueeMode;
pub use move_drag::MoveDrag;
pub use notes::{NoteNotice, NoteState, is_note_file};
pub use place::{PlaceDrag, Placing};
pub use placement::PagePlacement;
pub use resize_drag::ResizeDrag;
pub use scope::SelectionScope;
pub use tool::Tool;
pub use tool_defaults::{
    DrawDefaults, ShapeDefaults, StickyDefaults, TextDefaults, ToolDefaultPatch, ToolDefaults,
};
pub use update::update;
