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
mod arrange;
mod asset;
mod bindings;
mod camera;
mod caps;
mod chat;
mod clipboard;
mod clone;
mod comment;
mod cursor;
mod draw;
pub mod driver;
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
mod guides;
mod handles;
mod hit;
mod images;
mod layout;
mod live;
mod marquee;
mod menu;
mod move_drag;
mod notes;
mod page_input;
mod page_state;
mod pages;
pub mod panel;
mod place;
mod placement;
mod pointer;
pub mod property;
mod resize;
mod resize_drag;
mod reveal;
mod saved;
mod scope;
mod scroll_follow;
mod select;
mod select_all;
mod sidebar;
mod space;
mod stack_order;
mod strokes;
mod sync;
mod time;
mod tool;
mod tool_defaults;
mod update;
mod url;
mod verbs;
mod viewport;
mod zoom;

pub use anchor::{anchors_to_pages, matches_page_url};
pub use anchors::Anchor;
pub use api::{ApiCall, ApiOutcome, ApiRun};
pub use app::{App, Focus, Selection, Session};
pub use arrange::{ArrangeMode, arrange_command, place_command};
pub use asset::AssetBytes;
pub use bindings::{BINDINGS, Binding, Chord, Context, binding_for};
pub use caps::{AspectMode, aspect_mode, has_anchors, min_size};
pub use chat::{
    Bubble, CHAT_MAX_WIDTH, CHAT_MIN_WIDTH, CHAT_WIDTH, ChatAction, ChatModel, ChatView, Composer,
    DraftChip, DraftKind, ImageUpload, OpenComments, PillChip, PillKind, QueuedChip, RunBar,
    ThreadRow, Transcript, chat,
};
pub use clipboard::{ClipboardContent, ClipboardImage, Paste};
pub use comment::{
    CommentDrag, CommentMark, FOCUS_RING_OUTSET, FOCUS_RING_STROKE, MarkShape, PILL_DIGIT_WIDTH,
    PILL_EDGE_MARGIN, PILL_HEIGHT, PILL_INSET, PILL_WIDTH, PageGrab, PageRegion, REGION_HIT_BAND,
    REGION_MIN_SIZE, element_on_canvas, left_its_page, page_clip, region_annotation,
    region_on_canvas, selection_metadata,
};
pub use draw::DrawStroke;
pub use driver::Driver;
pub use drop::{DroppedFile, default_size as dropped_size, shown_path};
pub use edge_drag::{EdgeDrag, EdgePreview};
pub use edge_path::EdgeCurve;
pub use edit::{
    CaretStop, EDGE_LABEL_SIZE, EditMarks, Format, LayoutLine, NOTE_PADDING, SourceLine, SourceRow,
    SourceSpan, SourceStyle, StackCache, TITLE_GAP, TITLE_LINE, TITLE_SIZE, TextEdit, TextFrame,
    TextLayout, TextMeasure, TextSelectDrag, TextSpec, note_frame, source_rows, style_lines,
};
pub use effect::{Cursor, Effect};
pub use event::{Action, CanvasAction, Event, Key, KeyInput, PageNotice, PointerInput, WheelInput};
pub use geometry::{ScreenRect, to_canvas_rect};
pub use gesture::Gesture;
pub use groups::group_command;
pub use guides::{
    AlignmentGuide, DistributionGap, DistributionGuide, GuideAxis, GuideReference, Guides,
};
pub use handles::{Corner, HANDLE_SIZE, Handle, HandleOwner, OUTLINE_PADDING};
pub use hit::{Hit, hit_test, title_scale};
pub use images::{Image, ImageKey, ImageNotice, ImageState, is_image_file};
pub use layout::Axis as LayoutAxis;
pub use layout::act::{gap_command, make_command as auto_layout_command, reorder_command};
pub use layout::drag::{LineDrag, ReorderGhost};
pub use layout::handles::{DOT_RADIUS, GapHandle, LayoutHandle, ReorderDot};
pub use marquee::MarqueeMode;
pub use menu::{Menu, MenuEntry, MenuItem, binding_of, menus};
pub use move_drag::{CopyPreview, MoveDrag};
pub use notes::{NoteNotice, NoteState, is_note_file, note_file_name};
pub use page_state::PageState;
pub use panel::builtin::PanelUi;
pub use panel::{
    Align, Button, Choices, Control, ControlId, Dropdown, DropdownOption, DropdownSection, Entries,
    Face, Field, FieldSubmit, FieldWidth, Icon, Label, MenuTarget, OptionLayout, PaintRole,
    Palette, Placement, PopupAnchor, PopupModel, SidebarButton, Stepper, Swatch, Swatches, Toggle,
    ToolButton, ToolbarModel, ToolbarSection, context_menu, popup_for, toolbar,
};
pub use place::{PlaceDrag, Placing};
pub use placement::PagePlacement;
pub use property::{Orientation, Property};
pub use resize_drag::ResizeDrag;
pub use scope::SelectionScope;
pub use scroll_follow::{
    Seen, doc_to_viewport, hittable_rect, left_page, recorded_scroll, seen, shift_of, shown_rect,
    viewport_to_doc,
};
pub use sidebar::{
    CanvasRow, RowKind, RowTarget, SIDEBAR_WIDTH, SectionHead, SidebarAction, SidebarModel,
    SidebarRow, SidebarSection, SidebarView, sidebar,
};
pub use space::{
    Canvas, CanvasId, DEFAULT_CANVAS_NAME, OpenedCanvas, OpenedSpace, Space, TabRefError,
    canvas_file_name, legacy_canvas_file_name, resolve_tab_ref,
};
pub use specular_agent::{
    Image as ThreadImage, Index as ThreadIndex, MediaType, Message as ThreadMessage, Notice,
    Progress, ProgressKind, Role as ThreadRole, RunRequest, RunState, Status as ThreadStatus,
    Thread, ThreadId, Threads,
};
pub use time::iso8601;
pub use tool::Tool;
pub use tool_defaults::{
    DrawDefaults, PageDefaults, ShapeDefaults, StickyDefaults, TextDefaults, ToolDefaultPatch,
    ToolDefaults,
};
pub use update::update;
pub use url::{looks_like_url, normalize_user_url, resolve_address_input};
pub use verbs::{delete_commands, move_commands};
pub use zoom::fitting as fit_camera;
