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
mod anchors;
mod api;
mod app;
mod asset;
mod bindings;
mod camera;
mod caps;
mod clipboard;
mod clone;
mod comment;
mod cursor;
mod draw;
mod drop;
mod edge_drag;
mod edge_path;
mod edit;
mod effect;
mod event;
mod focus;
mod geometry;
mod gesture;
mod grid;
mod group_drop;
mod group_fit;
mod groups;
mod handles;
mod hit;
mod images;
mod live;
mod marquee;
mod menu;
mod move_drag;
mod notes;
mod page_input;
mod page_state;
mod pages;
mod place;
mod placement;
mod pointer;
mod resize;
mod resize_drag;
mod saved;
mod scope;
mod scroll_follow;
mod select;
mod select_all;
mod stack_order;
mod strokes;
mod time;
mod tool;
mod tool_defaults;
mod update;
mod url;
mod verbs;
mod zoom;

pub use anchor::{anchors_to_pages, matches_page_url};
pub use anchors::Anchor;
pub use api::{ApiCall, ApiOutcome, ApiRun};
pub use app::{App, Focus, Selection, Session};
pub use asset::AssetBytes;
pub use bindings::{BINDINGS, Binding, Chord, Context, binding_for};
pub use caps::{AspectMode, aspect_mode, has_anchors, min_size};
pub use clipboard::{ClipboardContent, ClipboardImage, Paste};
pub use comment::{
    CommentDrag, CommentMark, FOCUS_RING_OUTSET, FOCUS_RING_STROKE, MarkShape, PILL_DIGIT_WIDTH,
    PILL_EDGE_MARGIN, PILL_HEIGHT, PILL_INSET, PILL_WIDTH, PageGrab, PageRegion, REGION_HIT_BAND,
    REGION_MIN_SIZE, element_on_canvas, left_its_page, page_clip, region_annotation,
    region_on_canvas, selection_metadata,
};
pub use draw::DrawStroke;
pub use drop::DroppedFile;
pub use edge_drag::{EdgeDrag, EdgePreview};
pub use edge_path::EdgeCurve;
pub use edit::{
    CaretStop, EDGE_LABEL_SIZE, Format, LayoutLine, NOTE_PADDING, SourceLine, SourceRow,
    SourceSpan, SourceStyle, TITLE_GAP, TITLE_LINE, TITLE_SIZE, TextEdit, TextFrame, TextLayout,
    TextMeasure, TextSelectDrag, TextSpec, note_frame, source_rows, style_lines,
};
pub use effect::{Cursor, Effect};
pub use event::{Action, Event, Key, KeyInput, PageNotice, PointerInput, WheelInput};
pub use geometry::{ScreenRect, to_canvas_rect};
pub use gesture::Gesture;
pub use groups::group_command;
pub use handles::{Corner, HANDLE_SIZE, Handle, HandleOwner, OUTLINE_PADDING};
pub use hit::{Hit, hit_test, title_scale};
pub use images::{Image, ImageKey, ImageNotice, ImageState, is_image_file};
pub use marquee::MarqueeMode;
pub use menu::{Menu, MenuEntry, MenuItem, binding_of, menus};
pub use move_drag::{CopyPreview, MoveDrag};
pub use notes::{NoteNotice, NoteState, is_note_file};
pub use page_state::PageState;
pub use place::{PlaceDrag, Placing};
pub use placement::PagePlacement;
pub use resize_drag::ResizeDrag;
pub use scope::SelectionScope;
pub use scroll_follow::{
    Seen, doc_to_viewport, hittable_rect, left_page, recorded_scroll, seen, shift_of, shown_rect,
    viewport_to_doc,
};
pub use time::iso8601;
pub use tool::Tool;
pub use tool_defaults::{
    DrawDefaults, ShapeDefaults, StickyDefaults, TextDefaults, ToolDefaultPatch, ToolDefaults,
};
pub use update::update;
pub use url::{looks_like_url, normalize_user_url};
pub use verbs::{delete_commands, move_commands};
pub use zoom::fitting as fit_camera;
