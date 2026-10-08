//! [`Effect`]: the I/O [`update`](crate::update) asks the shell to do.

use glam::Vec2;
use specular_agent::{RunRequest, ThreadId};
use specular_core::{CssSize, InputEvent, LocatorBundle, PageNav, PointKind};
use specular_doc::{ColorScheme, EntityId, Rect};

use crate::{
    ApiOutcome, AppSettings, AssetBytes, CanvasId, ImageKey, PageRegion, Theme, ToolDefaults,
};

/// One thing for the shell to do after an [`update`](crate::update). Effects
/// run in the order they are returned.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Answer the [`Event::Api`](crate::Event::Api) that carried `ticket`,
    /// once the event's other effects have run. The document and the
    /// session are by then as the call left them.
    ApiReply {
        /// The call's ticket.
        ticket: u64,
        /// How it went.
        outcome: ApiOutcome,
    },
    /// Start hosting a page entity.
    CreatePage {
        /// The page entity.
        page: EntityId,
        /// The URL to load.
        url: String,
        /// The layout viewport in CSS pixels.
        viewport: CssSize,
    },
    /// Stop hosting a page entity.
    ClosePage(EntityId),
    /// Re-lay-out a hosted page at a new viewport.
    SetPageViewport {
        /// The page entity.
        page: EntityId,
        /// The new layout viewport in CSS pixels.
        viewport: CssSize,
    },
    /// Move a hosted page through its history, or to another address. The
    /// page stays hosted: it keeps its history, its focus and its frames.
    Navigate {
        /// The page entity.
        page: EntityId,
        /// Where to.
        nav: PageNav,
    },
    /// Ask a hosted page how far its document is scrolled, as a fraction of
    /// how far it can scroll. The answer is a
    /// [`PageNotice::ScrollProgress`](crate::PageNotice::ScrollProgress).
    AskScrollProgress(EntityId),
    /// Scroll a hosted page's document to a fraction of how far it can
    /// scroll: what keeps the pages of a sync set at the same place in
    /// their content whatever their widths.
    ScrollPage {
        /// The page entity.
        page: EntityId,
        /// The fraction along x and y, each in `0..=1`.
        progress: Vec2,
    },
    /// Report the hovers and clicks the user gives this page, as
    /// [`PageNotice::Pointed`](crate::PageNotice::Pointed), and no other
    /// page's. `None` stops the reports.
    CapturePage(Option<EntityId>),
    /// Ask a hosted page for the elements `bundle` could mean. The answer
    /// is a [`PageNotice::Candidates`](crate::PageNotice::Candidates) with
    /// this `request`.
    AskCandidates {
        /// The page entity.
        page: EntityId,
        /// What the answer repeats.
        request: u64,
        /// The element another page was pointed at.
        bundle: Box<LocatorBundle>,
    },
    /// Replay a hover or a click on a hosted page as trusted input.
    ReplayPointer {
        /// The page entity.
        page: EntityId,
        /// A move or a click.
        kind: PointKind,
        /// Where, in the page's viewport CSS pixels.
        point: Vec2,
    },
    /// Give a page keyboard focus, or take it from every page.
    FocusPage(Option<EntityId>),
    /// Send one input event into a page, in its CSS pixels.
    ForwardInput {
        /// The page entity.
        page: EntityId,
        /// The event.
        event: InputEvent,
    },
    /// Turn the OS input method on or off for the window.
    SetImeAllowed(bool),
    /// Put the OS candidate window next to this rect, in logical screen
    /// pixels.
    SetImeCursorArea {
        /// Top-left corner.
        origin: Vec2,
        /// Width and height.
        size: Vec2,
    },
    /// Show this cursor over the canvas.
    SetCursor(Cursor),
    /// The active canvas changed: write it to its file once changes stop.
    Save,
    /// Write a canvas of the space to its file now, making the file if
    /// there is none: a new canvas, a copy, or a background canvas that
    /// changed. What to write is
    /// [`App::canvas_to_save`](crate::App::canvas_to_save).
    WriteCanvas(CanvasId),
    /// A canvas was renamed: move its file, inside the space folder. What
    /// is unsaved goes to the old name first.
    RenameCanvasFile {
        /// The canvas.
        canvas: CanvasId,
        /// The file's name until now.
        from: String,
        /// Its name from now on.
        to: String,
    },
    /// A canvas was deleted: send its file to the system trash and stop
    /// following it.
    TrashCanvasFile {
        /// The canvas, which the space no longer lists.
        canvas: CanvasId,
        /// Its file's name inside the space folder.
        file: String,
    },
    /// The list of canvases or the active one changed: write the space's
    /// index from [`App::space`](crate::App::space).
    SaveSpaceMeta,
    /// Put text on the system clipboard: the selected text of an edit, or
    /// the selected entities as [`copy`](crate::Action::Copy) writes them.
    WriteClipboard(String),
    /// Read the system clipboard and answer with
    /// [`Event::Clipboard`](crate::Event::Clipboard).
    ReadClipboard,
    /// Write `bytes` to a new file in the space folder. It comes before the
    /// effect that loads the file.
    WriteAsset {
        /// Where, relative to the space folder.
        file: String,
        /// The file's contents.
        bytes: AssetBytes,
    },
    /// Copy a file from outside the space folder into it. It comes before
    /// the effect that loads the copy.
    CopyAsset {
        /// The absolute path of the file to copy.
        from: String,
        /// Where the copy goes, relative to the space folder.
        file: String,
    },
    /// Decode an image file and upload it under `image`, then answer with
    /// [`Event::Image`](crate::Event::Image).
    LoadImage {
        /// The key the answer and the renderer use.
        image: ImageKey,
        /// The path as the document writes it: relative to the space folder,
        /// absolute, or a URL.
        file: String,
    },
    /// Draw the svg at `image` again at a new size and upload it over the
    /// old raster, then answer with [`Event::Image`](crate::Event::Image).
    /// The size is what the svg is drawn at in logical pixels, which the
    /// shell scales to the window's pixels.
    RasterImage {
        /// The key of the svg's earlier load.
        image: ImageKey,
        /// The path as the document writes it.
        file: String,
        /// What to draw at, in logical pixels.
        size: specular_core::PixelSize,
    },
    /// Forget an image: nothing shows it any more.
    DropImage(ImageKey),
    /// Read a markdown file and answer with
    /// [`Event::Note`](crate::Event::Note), then again whenever the file
    /// changes on disk.
    LoadNote {
        /// The path as the document writes it, which the answer repeats.
        file: String,
    },
    /// Write `text` to a markdown file, making the file if there is none.
    /// The shell must not overwrite a text it has not seen: if the file
    /// holds something other than what was last read from it or written to
    /// it, the write is left out and answered with
    /// [`NoteNotice::Refused`](crate::NoteNotice::Refused).
    WriteNote {
        /// The path as the document writes it.
        file: String,
        /// The whole text of the file.
        text: String,
    },
    /// Make an empty markdown file in the space folder under a name no file
    /// there has, and answer with
    /// [`Event::NoteCreated`](crate::Event::NoteCreated).
    CreateNote {
        /// Where the Document for the file goes, which the answer repeats.
        rect: Rect,
    },
    /// Stop watching a markdown file: nothing shows it any more.
    DropNote {
        /// The path as the document writes it.
        file: String,
    },
    /// Find the element a page has under a point and answer with
    /// [`Event::ElementAt`](crate::Event::ElementAt), which repeats the page
    /// and the point. An answer of no element is still an answer.
    QueryElement {
        /// The page entity.
        page: EntityId,
        /// The point, in the page's viewport CSS pixels.
        point: Vec2,
    },
    /// Read the node a page has under a point, as the inspect tool shows it,
    /// and answer with a
    /// [`PageNotice::Inspected`](crate::PageNotice::Inspected) that repeats
    /// the point and `pick`. An answer of no node is still an answer.
    InspectAt {
        /// The page entity.
        page: EntityId,
        /// The point, in the page's viewport CSS pixels.
        point: Vec2,
        /// Whether a click asked, so the node becomes the selected one. A
        /// hover asks with `false`.
        pick: bool,
    },
    /// Write the connected repos, as
    /// [`Repos::to_json`](specular_agent::Repos::to_json) of
    /// [`App::repos`](crate::App::repos) gives them, to this app's own
    /// `repos.json`.
    SaveRepos,
    /// Ask the user for a folder and answer with an
    /// [`Action::Repo`](crate::Action::Repo): a
    /// [`RepoAction::Bind`](crate::RepoAction::Bind) of `origin` to it, or a
    /// [`RepoAction::Connect`](crate::RepoAction::Connect) when there is no
    /// origin. A cancelled dialog answers nothing.
    PickRepoFolder {
        /// The origin to bind to the folder chosen.
        origin: Option<String>,
    },
    /// Ask the user for a folder, open it as the space and remember it for
    /// the next launch. The space that was open is saved first and left as
    /// it is. A cancelled dialog changes nothing.
    ChooseSpace {
        /// Whether the dialog is worded for making a space.
        create: bool,
    },
    /// Open the folder at this path as the space and remember it.
    OpenSpace(String),
    /// Show the open space's folder in the file manager.
    RevealSpace,
    /// Quit the app.
    Quit,
    /// Write the settings to the preferences file, under `show`.
    SaveSettings(AppSettings),
    /// Count the elements each page has inside a comment region and answer
    /// with [`Event::RegionGrab`](crate::Event::RegionGrab), which repeats
    /// the region and lists the pages in this order.
    QueryRegionGrab {
        /// The region, in canvas space.
        region: Rect,
        /// The pages the region lies over, front to back, each with the
        /// part of it the region covers.
        pages: Vec<PageRegion>,
    },
    /// Ask a page which element an item centred on a point of its document
    /// should follow (ADR 0032), and answer with a
    /// [`PageNotice::ElementCaptured`](crate::PageNotice::ElementCaptured)
    /// carrying `request`. A page that cannot be asked is not answered for.
    CaptureElement {
        /// The page entity.
        page: EntityId,
        /// The number the answer repeats.
        request: u64,
        /// The point, in the page's document CSS pixels.
        point: Vec2,
    },
    /// Tell a page which elements anchored items follow. It answers with a
    /// [`PageNotice::ElementPlaces`](crate::PageNotice::ElementPlaces) for
    /// all of them, and again for each that moves. Replaces the set named
    /// before.
    TrackElements {
        /// The page entity.
        page: EntityId,
        /// The elements' selectors, sorted.
        selectors: Vec<String>,
    },
    /// Write the tool defaults to the preferences file, under `toolDefaults`,
    /// as [`ToolDefaults::to_json`] gives them.
    SaveToolDefaults(Box<ToolDefaults>),
    /// Write the theme choice to the preferences file, under `themeMode`,
    /// as [`Theme::key`] names it.
    SaveTheme(Theme),
    /// Tell a hosted page which `prefers-color-scheme` to report
    /// (`applyPageColorScheme`). `scheme` is the page's own setting; `None`
    /// follows the app, so the shell resolves it with
    /// [`App::appearance`](crate::App::appearance). Asked when a page is
    /// made, when its setting changes and when the app's appearance does.
    SetPageColorScheme {
        /// The page entity.
        page: EntityId,
        /// The page's own scheme, if it has one.
        scheme: Option<ColorScheme>,
    },
    /// Read every thread file under `.specular/threads/` of the space
    /// folder, and `index.json` beside them, and answer with
    /// [`Event::ThreadsLoaded`](crate::Event::ThreadsLoaded). Asked when a
    /// space with a folder opens. Files that are not threads are skipped.
    LoadThreads,
    /// Write one thread to `.specular/threads/<canvas id>/<thread id>.json`
    /// in the space folder. The text is
    /// [`Thread::to_json`](specular_agent::Thread::to_json) of
    /// `app.threads().get(id)`; a thread that is gone writes nothing.
    WriteThread(ThreadId),
    /// Write `.specular/threads/index.json` in the space folder, from
    /// [`App::thread_index_json`](crate::App::thread_index_json).
    WriteThreadIndex,
    /// Start the `claude` CLI for a thread, in the request's `cwd` or else
    /// the space folder, and answer
    /// with an [`Event::Agent`](crate::Event::Agent) for each thing its
    /// output says, the last being a
    /// [`Notice::Finished`](specular_agent::Notice::Finished) or a
    /// [`Notice::Failed`](specular_agent::Notice::Failed). The images the
    /// request lists were written (by a [`WriteAsset`](Self::WriteAsset)
    /// before this effect) relative to the space folder.
    RunAgent(Box<RunRequest>),
    /// Stop a thread's run and answer with
    /// [`Notice::Cancelled`](specular_agent::Notice::Cancelled).
    CancelAgent(ThreadId),
}

/// A pointer cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Cursor {
    /// The arrow.
    #[default]
    Default,
    /// A creation tool is armed.
    Crosshair,
    /// The canvas can be dragged.
    Grab,
    /// The canvas is being dragged.
    Grabbing,
    /// An item can be moved.
    Move,
    /// A text caret.
    Text,
    /// A top-left or bottom-right resize handle.
    ResizeNwse,
    /// A top-right or bottom-left resize handle.
    ResizeNesw,
}
