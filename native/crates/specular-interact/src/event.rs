//! [`Event`]: everything that can happen to an [`App`](crate::App).

use glam::Vec2;
use specular_agent::{Index, Notice, Repos, Thread, ThreadId};
use specular_core::{
    Camera, CapturedElement, ElementPlace, ImeEvent, InspectedNode, LocatorBundle,
    LocatorCandidate, Modifiers, PageElement, PixelRect, PointKind, PointerEventKind,
};
use specular_doc::{AnnotationId, Document, EntityId, ItemId, Rect};

use crate::{
    AboutRow, ApiCall, AppSettings, ArrangeMode, CanvasId, ChatAction, ClipboardContent,
    DroppedFile, Format, ImageKey, ImageNotice, NoteNotice, OpenedSpace, PageGrab, Property,
    RepoAction, SettingAction, SidebarAction, SpaceAction, SpaceAsk, Tool, ToolDefaultPatch,
    ToolDefaults,
};

/// One input to [`update`](crate::update). Window input arrives in logical
/// screen pixels, origin at the canvas viewport's top-left.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The pointer moved, pressed, released or left the window.
    Pointer(PointerInput),
    /// A wheel or two-finger scroll.
    Wheel(WheelInput),
    /// A trackpad pinch. `delta` is the change in magnification; positive
    /// zooms in.
    Pinch {
        /// The change in magnification.
        delta: f32,
    },
    /// A key went down or up.
    Key(KeyInput),
    /// The OS input method composed or committed text.
    Ime(ImeEvent),
    /// A hosted page reported something.
    Page {
        /// The page entity.
        page: EntityId,
        /// What it reported.
        notice: PageNotice,
    },
    /// The shell finished with an image an
    /// [`Effect::LoadImage`](crate::Effect::LoadImage) asked for.
    Image {
        /// The image.
        image: ImageKey,
        /// How it went.
        notice: ImageNotice,
    },
    /// The shell read a markdown file an
    /// [`Effect::LoadNote`](crate::Effect::LoadNote) asked for, or saw it
    /// change on disk.
    Note {
        /// The path as the document writes it.
        file: String,
        /// What the file holds now.
        notice: NoteNotice,
    },
    /// The shell made the file an
    /// [`Effect::CreateNote`](crate::Effect::CreateNote) asked for.
    NoteCreated {
        /// The new file, relative to the space folder.
        file: String,
        /// Where its Document goes.
        rect: Rect,
    },
    /// The renderer stacked the rows of the Documents it drew and found how
    /// tall each one's text is, in canvas units. Sent when a height changes.
    NoteHeights(Vec<(EntityId, f32)>),
    /// The wall clock, sent once per loop turn. Milliseconds since the Unix
    /// epoch.
    Tick {
        /// The time.
        unix_ms: u64,
    },
    /// The canvas viewport changed size, in logical pixels.
    ViewportResized(Vec2),
    /// A document was loaded: at startup, on switching canvas, or when the
    /// file changed on disk. Replaces the current one and clears the history.
    DocumentOpened(Box<Document>),
    /// A space folder was read: at startup, or when another is chosen.
    /// Replaces every canvas and shows the space's active one.
    SpaceOpened(Box<OpenedSpace>),
    /// The file of one canvas of the space changed on disk and was read
    /// again. Replaces that canvas's document and clears its history,
    /// whether it is the active one or not.
    CanvasFileChanged {
        /// The canvas.
        canvas: CanvasId,
        /// What its file holds now.
        document: Box<Document>,
    },
    /// What the system clipboard holds: the answer to an
    /// [`Effect::ReadClipboard`](crate::Effect::ReadClipboard). Its text goes
    /// into the text being edited. With no edit open,
    /// [`Paste::of`](crate::Paste::of) decides what lands on the canvas.
    Clipboard(ClipboardContent),
    /// Files were dropped on the canvas.
    FilesDropped {
        /// The files, in the order they arrived.
        files: Vec<DroppedFile>,
        /// Where they were dropped, in logical screen pixels, when the shell
        /// knows.
        screen: Option<Vec2>,
    },
    /// What a page has under a point: the answer to an
    /// [`Effect::QueryElement`](crate::Effect::QueryElement). The comment
    /// clicked there becomes a draft on the element, or on the canvas point
    /// when there is none.
    ElementAt {
        /// The page entity.
        page: EntityId,
        /// The point asked about, in the page's viewport CSS pixels.
        point: Vec2,
        /// The element there, if the page has one.
        element: Option<PageElement>,
    },
    /// What a comment region grabbed in the pages it lies over: the answer
    /// to an [`Effect::QueryRegionGrab`](crate::Effect::QueryRegionGrab).
    /// The region becomes a draft in the first page it grabbed an element
    /// of, or on the canvas when it grabbed none.
    RegionGrab {
        /// The region asked about, in canvas space.
        region: Rect,
        /// What it grabbed in each page, front to back.
        grabs: Vec<PageGrab>,
    },
    /// The tool defaults were read from the preferences file. Replaces the
    /// current ones and asks for no save.
    ToolDefaultsLoaded(Box<ToolDefaults>),
    /// The settings were read from the preferences file. Replaces the
    /// current ones, shows what they say is shown at launch, and asks for
    /// no save.
    SettingsLoaded(AppSettings),
    /// What the app is built from, for the settings dialog's About pane.
    /// A shell sends it once at startup.
    About(Vec<AboutRow>),
    /// The launch found no space to open: none was ever chosen, or the one
    /// chosen is not there. The app shows the first-run view until a
    /// [`SpaceOpened`](Self::SpaceOpened).
    SpaceNeeded(SpaceAsk),
    /// The theme choice was read from the preferences file. Replaces the
    /// current one and asks for no save.
    ThemeLoaded(crate::Theme),
    /// The operating system's appearance, sent at startup and whenever it
    /// changes. It is what the `System` theme draws.
    SystemAppearance(crate::Appearance),
    /// A command from a key binding, a menu or a panel.
    Action(Action),
    /// A click on the control with this name, from a caller that has no
    /// layout to click in: a script or a test run with the panels off. The
    /// names are the models' ([`named_controls`](crate::named_controls)),
    /// and a name no control has does nothing. The keys are the ones held,
    /// which a row of the sidebar reads.
    Control(crate::ControlId, Modifiers),
    /// A right press at this screen point from such a caller: the context
    /// menu opens for what is there, to be picked from by name.
    ContextMenu(Vec2),
    /// Turns the built-in chrome on or off. A shell that draws it through
    /// `specular-scene` sends `true` once at startup; one that draws the
    /// panel models itself never does.
    BuiltinPanels(bool),
    /// Turns on only the built-in context menu. For a shell whose UI
    /// library draws the chrome and the sidebar, while the menu a right
    /// press on the canvas opens stays in the canvas's own pass.
    BuiltinMenu,
    /// Whether the shell draws a right panel. A shell that does sends `true`
    /// once at startup: a comment draft is then finished in the panel's
    /// field, and the canvas draws only its marker. Without it the draft is
    /// written in a card on the canvas.
    ChatPanel(bool),
    /// The threads of the space folder: the answer to an
    /// [`Effect::LoadThreads`](crate::Effect::LoadThreads). Replaces every
    /// thread held.
    ThreadsLoaded {
        /// The threads read, closed ones included.
        threads: Vec<Thread>,
        /// `index.json`, or the empty index when there is none.
        index: Index,
    },
    /// A run reported something: output of an
    /// [`Effect::RunAgent`](crate::Effect::RunAgent), or the answer to an
    /// [`Effect::CancelAgent`](crate::Effect::CancelAgent).
    Agent {
        /// The thread being run.
        thread: ThreadId,
        /// What it said.
        notice: Notice,
    },
    /// The connected repos were read from `repos.json`. Replaces the ones
    /// held and asks for no save.
    ReposLoaded(Box<Repos>),
    /// A change the HTTP API asked for. It is answered with an
    /// [`Effect::ApiReply`](crate::Effect::ApiReply) carrying its ticket.
    Api(ApiCall),
}

/// A pointer event at a screen position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerInput {
    /// What the pointer did.
    pub kind: PointerEventKind,
    /// Where, in logical screen pixels. Unused for a leave.
    pub screen: Vec2,
    /// Modifier keys held.
    pub modifiers: Modifiers,
}

/// A scroll at the pointer's last position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelInput {
    /// The scroll in logical pixels. Positive `y` moves content down.
    pub delta: Vec2,
    /// Modifier keys held.
    pub modifiers: Modifiers,
}

/// One key transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInput {
    /// Which key, for bindings.
    pub key: Key,
    /// Down or up.
    pub pressed: bool,
    /// Whether this press is an auto-repeat.
    pub repeat: bool,
    /// The text the press produced, if any.
    pub text: Option<String>,
    /// The character the key carries to a page, on its press and on its
    /// release: what it types, or the platform's own character for a key
    /// that types nothing. `None` for a modifier key. A page host tells a
    /// key from a change of modifiers by it, so a key without one is not
    /// pressed and released in the page.
    pub character: Option<char>,
    /// Modifier keys held.
    pub modifiers: Modifiers,
    /// The Windows virtual-key code, which Chromium derives DOM `keyCode`
    /// from on every platform. Only used when the key goes to a page.
    pub windows_key_code: i32,
    /// The platform scan code, which Chromium derives DOM `code` from. Only
    /// used when the key goes to a page.
    pub native_key_code: i32,
}

/// The identity of a key for bindings: the physical key, so a binding sits
/// in the same place on every layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Escape.
    Escape,
    /// Return or keypad Enter.
    Enter,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Forward delete.
    Delete,
    /// Home.
    Home,
    /// End.
    End,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// The space bar.
    Space,
    /// Left arrow.
    ArrowLeft,
    /// Right arrow.
    ArrowRight,
    /// Up arrow.
    ArrowUp,
    /// Down arrow.
    ArrowDown,
    /// A letter, digit or punctuation key, as its unshifted US-layout
    /// character in lower case.
    Char(char),
    /// Any other key. It can still go to a page.
    Other,
}

/// Something a hosted page reported that the interaction layer acts on.
/// Painted frames are not here: they go straight to the renderer.
#[derive(Debug, Clone, PartialEq)]
pub enum PageNotice {
    /// The main frame finished loading.
    Loaded {
        /// The HTTP status, 0 for a non-HTTP load.
        http_status: i32,
    },
    /// The page's process went away.
    Crashed {
        /// The backend's reason, such as `crashed` or `oom`.
        reason: String,
    },
    /// The IME composition moved. Bounds are in the page's CSS pixels;
    /// `None` when there is no composition.
    ImeCompositionBounds(Option<PixelRect>),
    /// The document's title changed. Empty for a document with none.
    Title(String),
    /// The page shows another address: a navigation committed, or the page
    /// changed its own URL in place.
    Url(String),
    /// A load started or ended, or the page's history moved.
    Loading {
        /// Whether a load is in flight.
        loading: bool,
        /// Whether there is an entry to go back to.
        can_go_back: bool,
        /// Whether there is an entry to go forward to.
        can_go_forward: bool,
    },
    /// The document scrolled to this offset, in the page's CSS pixels.
    Scrolled {
        /// Along x.
        x: f64,
        /// Along y.
        y: f64,
    },
    /// How far the document is scrolled as a fraction of how far it can
    /// scroll: the answer to an
    /// [`Effect::AskScrollProgress`](crate::Effect::AskScrollProgress).
    ScrollProgress {
        /// Along x, in `0..=1`.
        x: f64,
        /// Along y, in `0..=1`.
        y: f64,
    },
    /// The captured page was pointed at by the user: a hover or a click
    /// on an element, described so that a peer can find its own.
    Pointed {
        /// A move or a click.
        kind: PointKind,
        /// The element.
        bundle: Box<LocatorBundle>,
    },
    /// The page's elements a bundle could mean: the answer to an
    /// [`Effect::AskCandidates`](crate::Effect::AskCandidates).
    Candidates {
        /// The request the question carried.
        request: u64,
        /// The elements.
        candidates: Vec<LocatorCandidate>,
    },
    /// The node the page has under a point: the answer to an
    /// [`Effect::InspectAt`](crate::Effect::InspectAt).
    Inspected {
        /// The point asked about, in the page's viewport CSS pixels.
        point: Vec2,
        /// Whether a click asked.
        pick: bool,
        /// The node there, if the page has one.
        node: Option<Box<InspectedNode>>,
    },
    /// The page's remote-debugging websocket is known.
    DevtoolsUrl(String),
    /// The element an item placed over the page should follow (ADR 0032):
    /// the answer to an
    /// [`Effect::CaptureElement`](crate::Effect::CaptureElement).
    ElementCaptured {
        /// The request the question carried.
        request: u64,
        /// The element, or `None` when the page has none to offer.
        element: Option<CapturedElement>,
    },
    /// Where elements the page was told to track
    /// ([`Effect::TrackElements`](crate::Effect::TrackElements)) sit in its
    /// document now, by selector. `None` for a selector that finds nothing.
    ElementPlaces(Vec<(String, Option<ElementPlace>)>),
}

/// A command with no pointer position: what a key binding, a menu item, a
/// toolbar button or an API route asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Escape, one stage a press. A comment draft is dropped; with none, a
    /// focused comment loses the focus. Otherwise: abandon the gesture in
    /// flight, return to the select tool and leave the entered page, and
    /// with none of those to back out of, clear the selection.
    Cancel,
    /// Switch tool.
    SetTool(Tool),
    /// Choose the theme and save the choice.
    SetTheme(crate::Theme),
    /// Change one tool default and save the defaults.
    SetToolDefault(ToolDefaultPatch),
    /// Switch to the tool a default belongs to and change that default: what
    /// a variant key such as Shift+R or Shift+M does.
    SetToolVariant(ToolDefaultPatch),
    /// Undo the latest document step. While text is being edited, the
    /// latest step of that edit instead.
    Undo,
    /// Redo the latest undone step, of the document or of the edit.
    Redo,
    /// Replace the selection. Ids that name nothing are dropped.
    Select(Vec<ItemId>),
    /// Select `select`, as [`Action::Select`] does, and bring `focus` into
    /// view: nothing moves when it is all in the part of the viewport the
    /// sidebar leaves free, and otherwise the camera pans, at the zoom it
    /// has, to centre it there. What a sidebar row sends.
    Reveal {
        /// What the selection becomes.
        select: Vec<ItemId>,
        /// What the camera shows.
        focus: ItemId,
    },
    /// Give a comment the focus, as [`Action::FocusComment`] does, and bring
    /// what it is on into view as [`Action::Reveal`] does.
    RevealComment(AnnotationId),
    /// Show or hide the sidebar, fold a section or open a row.
    Sidebar(SidebarAction),
    /// Move the camera.
    SetCamera(Camera),
    /// Remove the selection, with what is inside its groups, what is hooked
    /// to its pages and the edges that would lose an end.
    Delete,
    /// Copy the selection into free space beside it and select the copies.
    Duplicate,
    /// Put the selection on the clipboard.
    Copy,
    /// Put the selection on the clipboard and remove it.
    Cut,
    /// Paste the clipboard at the pointer.
    Paste,
    /// Select everything that is not inside a group.
    SelectAll,
    /// Zoom in one step about the middle of the viewport.
    ZoomIn,
    /// Zoom out one step about the middle of the viewport.
    ZoomOut,
    /// Zoom to 100% about the middle of the viewport.
    ZoomReset,
    /// Zoom to this many percent about the middle of the viewport. It names
    /// the level and not the camera it lands on, so the control that holds
    /// it is the same wherever the canvas is panned.
    ZoomTo(u16),
    /// Show everything on the canvas, centred.
    ZoomToFit,
    /// Toggle markdown formatting on the selection of the text being
    /// edited. Does nothing where the text does not take that format.
    Format(Format),
    /// Move the selection one slot forward in the stack order.
    BringForward,
    /// Move the selection one slot backward in the stack order.
    SendBackward,
    /// Move the selection in front of everything.
    BringToFront,
    /// Move the selection behind everything.
    SendToBack,
    /// Wrap the selected items in a new group and select it.
    Group,
    /// Take the selected group apart and select what was inside it.
    Ungroup,
    /// Open a comment draft on the region the selected entities span.
    AnnotateSelection,
    /// Lay the selected items out in a row, a column or a grid, keeping the
    /// footprint they have and evening the spacing inside it. Does nothing
    /// for fewer than two.
    Arrange(ArrangeMode),
    /// Make the selection an auto-layout group (ADR 0015) and select it: a
    /// lone group manages its members as a row or a column, whichever they
    /// are spread along, and two or more items are grouped first.
    AutoLayout,
    /// Pack the selected group's members along an axis, or with `None` let
    /// them sit where they are.
    GroupLayout(Option<crate::LayoutAxis>),
    /// Set the gap the selected auto-layout group packs with, in canvas
    /// units.
    GroupGap(f64),
    /// Show the canvas, or one page or Document alone. View state: nothing
    /// is written to the document and no undo step is made.
    Show(crate::Showing),
    /// Show the tab after the one showing, going round to the first.
    ShowNext,
    /// Show the tab before the one showing, going round to the last.
    ShowPrevious,
    /// Make a page at the page tool's preset in a free spot of the canvas,
    /// show it alone and put the caret in its address. The page is one undo
    /// step.
    NewPageTab,
    /// Put the caret in the address of the page whose controls the dock
    /// holds.
    EditPageUrl,
    /// Give a comment the focus, taking the selection away, or with `None`
    /// let go of the focus. An id that is not shown does nothing.
    FocusComment(Option<AnnotationId>),
    /// Mark a comment resolved. With `None`, every comment on the focused
    /// comment's mark, as one step. Does nothing with no such comment.
    ResolveComment(Option<AnnotationId>),
    /// Remove a comment. With `None`, every comment on the focused
    /// comment's mark, as one step. Does nothing with no such comment.
    DeleteComment(Option<AnnotationId>),
    /// Take the entered page, or the one selected page, an entry back in
    /// its history. Does nothing where there is none.
    PageBack,
    /// Take that page an entry forward in its history.
    PageForward,
    /// Load that page's address again.
    PageReload,
    /// Abandon that page's load in flight.
    PageStop,
    /// Take that page to an address, as one typed in its address field.
    PageNavigate(String),
    /// Toggle the sync set of the selected pages (ADR 0027). Two or more
    /// pages that already share one set leave it; otherwise they all join
    /// one new set. One selected page that is in a set leaves it. A set
    /// left with one page is dissolved. One undo step.
    ToggleSync,
    /// Change the space's canvases: show another, add, rename, copy or
    /// remove one.
    Canvas(CanvasAction),
    /// Something done in the right panel: its threads, its composer.
    Chat(ChatAction),
    /// A change to the connected repos and the origins bound to them.
    Repo(RepoAction),
    /// Choose, open or reveal the space folder, or quit from the first-run
    /// view.
    Space(SpaceAction),
    /// A change to what is kept between launches.
    Setting(SettingAction),
    /// Set one field of the selection, as a popup control does. It applies
    /// to every selected item it means something for, as one undo step.
    SetProperty(Property),
    /// Move the selection by exactly this many canvas units.
    Nudge {
        /// Along x. Positive is right.
        dx: f64,
        /// Along y. Positive is down.
        dy: f64,
    },
}

/// Something done to the canvases of the space.
#[derive(Debug, Clone, PartialEq)]
pub enum CanvasAction {
    /// Show another canvas. The one being left keeps its document, its
    /// undo history, its camera and its selection.
    Switch(CanvasId),
    /// Add an empty canvas named `Canvas N` and show it.
    New,
    /// Give a canvas another name, and its file with it. `None` is the
    /// active canvas. Does nothing when the name is empty or taken.
    Rename {
        /// The canvas.
        canvas: Option<CanvasId>,
        /// The new name.
        name: String,
    },
    /// Start typing a new name for a canvas in its row of the sidebar. `None`
    /// is the active canvas. Does nothing while the sidebar is hidden.
    BeginRename(Option<CanvasId>),
    /// Copy a canvas into a new one beside it and show the copy. `None` is
    /// the active canvas.
    Duplicate(Option<CanvasId>),
    /// Remove a canvas and send its file to the trash. `None` is the
    /// active canvas. The space's last canvas is replaced by an empty one.
    Delete(Option<CanvasId>),
}
