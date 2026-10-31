//! [`Event`]: everything that can happen to an [`App`](crate::App).

mod action;
mod input;
mod notice;

pub use self::action::{Action, CanvasAction};
pub use self::input::{Key, KeyInput, PointerInput, WheelInput};
pub use self::notice::PageNotice;

use glam::Vec2;
use specular_agent::{Index, Notice, Repos, Thread, ThreadId};
use specular_core::{ImeEvent, Modifiers, PageElement};
use specular_doc::{Document, EntityId, Rect};

use crate::{
    AboutRow, ApiCall, AppSettings, CanvasId, ClipboardContent, DroppedFile, ImageKey, ImageNotice,
    NoteNotice, OpenedSpace, PageGrab, SpaceAsk, ToolDefaults,
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
    /// The folder a document opened on its own sits in, which its relative
    /// file paths start from: an HTML file's page is loaded from there.
    DocumentFolder(String),
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
    /// The agent's models: the list the runtime gave, or the choice read
    /// from the preferences file. Neither asks for a save.
    Models(specular_agent::ModelsNews),
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
    /// The shell draws the chrome, the sidebar and the context menu. A
    /// right press on the canvas opens the menu's state, which the shell
    /// draws and picks from by name. A headless run never sends it.
    ShellChrome,
    /// The viewport is laid out as under the shell's chrome, the chrome
    /// and the docked tools covering its edges, by a caller that draws
    /// none: a headless script, so it means what it means in a window.
    ChromeLayout,
    /// The shell's context menu closed with nothing picked: a press outside
    /// it, or Escape while it had the keys.
    MenuClosed,
    /// Whether the shell draws a right panel. A shell that does sends `true`
    /// once at startup: a comment draft is then finished in the panel's
    /// field, and the canvas draws only its marker. Without it the draft is
    /// written in a card on the canvas.
    ChatPanel(bool),
    /// Whether camera transitions between the canvas, Device and Fill run.
    /// A windowed shell sends `true` once at startup; without it the camera
    /// lands at once.
    Motion(bool),
    /// Whether the system asks for reduced motion (macOS's Reduce motion).
    /// The shell reads it at startup and sends it again when it changes.
    /// With it on, transitions and travels land at once.
    ReduceMotion(bool),
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
    /// agent-browser is about to act on a page and waits for its cursor. It
    /// is answered with an [`Effect::AgentGo`](crate::Effect::AgentGo)
    /// carrying its ticket.
    AgentAct(crate::AgentAct),
    /// Sharing a canvas with peers, and what they send (ADR 0048).
    Share(crate::ShareEvent),
    /// A change the HTTP API asked for. It is answered with an
    /// [`Effect::ApiReply`](crate::Effect::ApiReply) carrying its ticket.
    Api(ApiCall),
}
