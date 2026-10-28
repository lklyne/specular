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
mod attach;
mod bindings;
mod camera;
mod caps;
mod chat;
mod chord;
mod clipboard;
mod clone;
mod comment;
mod cursor;
mod device;
mod draw;
pub mod driver;
mod drop;
mod edge_drag;
mod edge_path;
mod edit;
mod effect;
mod event;
mod first_run;
mod flick;
mod focus;
pub mod free_spot;
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
mod inspect;
mod labels;
mod layout;
mod link_open;
mod live;
mod mark;
mod marquee;
mod menu;
mod move_drag;
mod notes;
mod page_input;
mod page_state;
mod pages;
mod palette;
pub mod panel;
mod physical_key;
mod place;
mod placement;
mod pointer;
mod presence;
mod property;
mod repos;
mod resize;
mod resize_drag;
mod reveal;
mod safe_area;
mod saved;
mod scope;
mod scroll_follow;
mod select;
mod select_all;
mod settings;
mod share_button;
mod shared;
mod showing;
mod sidebar;
mod space;
mod space_file;
mod stack_order;
mod strokes;
mod sync;
mod theme;
mod time;
mod tool;
mod tool_defaults;
mod update;
mod url;
mod verbs;
mod viewport;
mod zoom;
mod zoom_settle;

pub use anchors::Anchor;
pub use api::{ApiCall, ApiOutcome, ApiRun};
pub use app::{App, Focus, Selection, Session};
pub use arrange::{ArrangeMode, arrange_command, place_command};
pub use asset::AssetBytes;
pub(crate) use bindings::page_edit;
pub use bindings::{BINDINGS, Binding, Context, SHELL_KEYS, ShellCommand, kept_from_page};
pub use chat::{
    ArtifactCard, AutoChip, Bubble, ChatAction, ChatModel, ChatView, Composer, DraftChip,
    DraftKind, FixTarget, ImageUpload, ModelChip, ModelOption, OpenComments, PillChip, PillKind,
    QueuedChip, RunBar, ThreadRow, Transcript, chat, fixable,
};
pub use chord::Chord;
pub use clipboard::{ClipboardContent, ClipboardImage, Paste};
pub use comment::{
    CommentDrag, CommentMark, FOCUS_RING_OUTSET, FOCUS_RING_STROKE, MarkShape, PILL_HEIGHT,
    PageGrab, PageRegion, element_on_canvas, left_its_page, page_clip, region_annotation,
    region_on_canvas, selection_metadata,
};
pub use draw::DrawStroke;
pub use driver::{Driver, EventSink};
pub use drop::{DroppedFile, default_size as dropped_size, shown_path};
pub use edge_drag::{EdgeDrag, EdgePreview};
pub use edge_path::EdgeCurve;
pub use edit::{
    CaretStop, EDGE_LABEL_SIZE, EditMarks, Format, LayoutLine, NOTE_PADDING, SourceRow, SourceSpan,
    SourceStyle, StackCache, TITLE_GAP, TITLE_LINE, TITLE_SIZE, TextEdit, TextFrame, TextLayout,
    TextMeasure, TextSelectDrag, TextSpec, source_rows,
};
pub use effect::Effect;
pub use event::{Action, CanvasAction, Event, Key, KeyInput, PageNotice, PointerInput, WheelInput};
pub use first_run::{OnboardingModel, SpaceAction, SpaceAsk, SpaceChoice, onboarding};
pub use geometry::{ScreenRect, to_canvas_rect};
pub use gesture::Gesture;
pub use groups::group_command;
pub use guides::{
    AlignmentGuide, DistributionGap, DistributionGuide, GuideAxis, GuideReference, Guides,
};
pub use handles::{Corner, HANDLE_SIZE, Handle, HandleOwner, OUTLINE_PADDING};
pub use hit::{Hit, hit_test, title_scale};
pub use images::{Image, ImageKey, ImageNotice, ImageState, is_image_file};
pub use inspect::{InspectFont, InspectModel, InspectPopover, InspectSwatch, InspectTarget};
pub use layout::Axis as LayoutAxis;
pub use layout::act::{make_command as auto_layout_command, reorder_command};
pub use layout::drag::{LineDrag, ReorderGhost};
pub use layout::handles::{DOT_RADIUS, GapHandle, LayoutHandle, ReorderDot};
pub use mark::Mark;
pub use marquee::MarqueeMode;
pub(crate) use menu::binding_of;
pub use menu::{Menu, MenuEntry, MenuItem, menus};
pub use move_drag::{CopyPreview, MoveDrag};
pub use notes::{NoteNotice, NoteState, is_note_file, note_file_name};
pub use page_state::{PageState, ScreenFit};
pub use palette::{PaletteItem, PaletteMatch, PaletteRun, palette, search};
pub use panel::PanelUi;
pub use panel::{
    Button, Choices, Control, ControlId, ControlsModel, Dropdown, DropdownOption, DropdownSection,
    Entries, Face, Field, FieldSubmit, FieldWidth, Icon, Label, MenuTarget, OptionLayout,
    PaintRole, Palette, ShareButton, ShareState, SidebarButton, Stepper, Swatch, Swatches,
    ThemeButton, Toggle, ToolButton, ToolbarModel, ToolbarSection, UnknownControl, ViewStrip,
    ViewTab, context_menu, control_named, dock, tool_options, toolbar, view_strip,
};
pub use physical_key::{PhysicalKey, mac_key_input};
pub use place::{PlaceDrag, PlacePreview};
pub use placement::PagePlacement;
pub use presence::{AgentAct, AgentCursor};
pub use property::{Orientation, PageStep, Property};
pub use repos::{BoundOriginRow, RepoAction, RepoRow, ReposPane, repos_pane};
pub use resize_drag::ResizeDrag;
pub use safe_area::within as within_safe_area;
pub use scope::SelectionScope;
pub use scroll_follow::{PAGE_FADE, Seen, seen, shown_rect};
pub(crate) use scroll_follow::{
    doc_to_viewport, hittable_rect, out_of_page, recorded_scroll, viewport_to_doc,
};
pub use settings::{
    AboutRow, AppSettings, GeneralPane, HiddenCursors, SettingAction, SettingChoice, SettingOption,
    SettingToggle, SettingsModel, ShortcutRow, SpaceRow, TOOLS_DOCK_WIDTH, ToolsPlace, settings,
};
pub use share_button::{LinkScope, ShareAction, ShareLinks};
pub use shared::ShareEvent;
pub use showing::turn::Turning;
pub use showing::{Lens, SavedTab, Showing};
pub use sidebar::{
    CanvasRow, RowKind, RowTarget, SIDEBAR_WIDTH, SectionHead, SidebarAction, SidebarModel,
    SidebarRow, SidebarSection, SidebarView, sidebar,
};
pub use space::{
    Canvas, CanvasId, OpenedCanvas, OpenedSpace, SavedCanvas, Space, TabRefError, canvas_file_name,
    legacy_canvas_file_name,
};
pub use space_file::{file_url_path, resolve_file};
pub use specular_agent::{
    Artifact, ArtifactTarget, Image as ThreadImage, Index as ThreadIndex, MediaType,
    Message as ThreadMessage, Model, Models, ModelsNews, Notice, Progress, ProgressKind, Repos,
    Role as ThreadRole, RunRequest, RunState, Status as ThreadStatus, Thread, ThreadId, Threads,
    WriteTarget, origin_of,
};
pub use specular_core::Cursor;
pub(crate) use theme::ThemeState;
pub use theme::{Appearance, Theme};
pub use time::iso8601;
pub use tool::Tool;
pub use tool_defaults::{
    DrawDefaults, PageDefaults, ShapeDefaults, StickyDefaults, TextDefaults, ToolDefaultPatch,
    ToolDefaults,
};
pub use update::update;
pub(crate) use url::resolve_address_input;
pub use verbs::{delete_commands, move_commands};
pub use zoom::fitting as fit_camera;
pub use zoom_settle::ZOOM_SETTLE_MS;
