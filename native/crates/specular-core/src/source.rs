//! The page backend contract.

use std::sync::Arc;

use glam::Vec2;

use crate::frame::FrameEvent;
use crate::geometry::{CssRect, CssSize, PixelRect};
use crate::input::InputEvent;
use crate::locator::{LocatorBundle, LocatorCandidate};
use crate::page::{PageId, PageSpec};

/// Errors a [`PageSource`] reports.
#[derive(Debug, thiserror::Error)]
pub enum PageSourceError {
    /// The id is unknown to this source (never created, or already closed).
    #[error("unknown page {0}")]
    UnknownPage(PageId),
    /// The spec cannot be hosted (empty viewport, non-positive scale, bad URL).
    #[error("invalid page spec: {0}")]
    InvalidSpec(String),
    /// The backend (CEF) refused or failed the call.
    #[error("page backend error: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Something that happened to a page, drained by [`PageSource::drain_events`].
#[derive(Debug)]
pub enum PageEvent {
    /// A layer painted.
    Frame(FrameEvent),
    /// The source refused a shared-texture paint because the page already
    /// held [`MAX_OUTSTANDING_TEXTURES`](crate::MAX_OUTSTANDING_TEXTURES)
    /// (Electron's `framesDroppedForPoolPressure`).
    FrameDropped {
        /// The page.
        page: PageId,
    },
    /// A popup widget opened or closed (CEF `OnPopupShow`). While hidden, the
    /// compositor drops the page's popup layer.
    PopupVisibility {
        /// The page.
        page: PageId,
        /// Whether the popup is now shown.
        visible: bool,
    },
    /// A popup widget moved/resized (CEF `OnPopupSize`), in view texels.
    PopupRect {
        /// The page.
        page: PageId,
        /// New placement within the view frame.
        rect: PixelRect,
    },
    /// The IME caret/composition bounds changed (CEF
    /// `OnImeCompositionRangeChanged`), so the app can place the candidate
    /// window. Union of character bounds, in page-local CSS pixels.
    ImeCompositionBounds {
        /// The page.
        page: PageId,
        /// Bounds of the composition, or `None` when there is none.
        bounds: Option<PixelRect>,
    },
    /// Main-frame load finished (HTTP status, 0 for non-HTTP).
    Loaded {
        /// The page.
        page: PageId,
        /// HTTP status code.
        http_status: i32,
    },
    /// The page host went away without being closed (renderer crash/OOM).
    Crashed {
        /// The page.
        page: PageId,
        /// Backend-provided reason, e.g. `crashed`, `oom`, `killed`.
        reason: String,
    },
    /// The document's title changed (CEF `OnTitleChange`).
    Title {
        /// The page.
        page: PageId,
        /// The new title; empty when the document has none.
        title: String,
    },
    /// The main frame shows another address (CEF `OnAddressChange`): a
    /// navigation committed, or the page changed its own URL in place.
    Url {
        /// The page.
        page: PageId,
        /// The address now shown.
        url: String,
    },
    /// The page started or stopped loading, or its session history moved
    /// (CEF `OnLoadingStateChange`).
    Loading {
        /// The page.
        page: PageId,
        /// Whether a load is in flight.
        loading: bool,
        /// Whether [`PageNav::Back`] has somewhere to go.
        can_go_back: bool,
        /// Whether [`PageNav::Forward`] has somewhere to go.
        can_go_forward: bool,
    },
    /// The main frame scrolled (CEF `OnScrollOffsetChanged`).
    Scrolled {
        /// The page.
        page: PageId,
        /// The document's scroll offset, in CSS pixels.
        offset: Vec2,
    },
    /// The answer to a [`PageSource::scroll_progress`].
    ScrollProgress {
        /// The page.
        page: PageId,
        /// How far the document is scrolled along each axis, as a fraction
        /// of how far it can scroll: 0 at the start, 1 at the end, and 0
        /// along an axis that does not scroll.
        progress: Vec2,
    },
    /// The captured page ([`PageSource::set_capture`]) was pointed at by
    /// real input: the pointer moved onto or within an element, or clicked
    /// one.
    Pointed {
        /// The page.
        page: PageId,
        /// Whether it was a move or a click.
        kind: PointKind,
        /// The element, described so that another page can find its own.
        bundle: Box<LocatorBundle>,
    },
    /// The answer to a [`PageSource::query_candidates`].
    Candidates {
        /// The page.
        page: PageId,
        /// The `request` the question carried.
        request: u64,
        /// The page's visible elements, or the one that carries the
        /// bundle's id.
        candidates: Vec<LocatorCandidate>,
    },
    /// The answer to a [`PageSource::query_element`].
    ElementAt {
        /// The page.
        page: PageId,
        /// The `request` the question carried.
        request: u64,
        /// The element under the point, or `None` when nothing is there.
        element: Option<PageElement>,
    },
    /// The answer to a [`PageSource::inspect_at`].
    Inspected {
        /// The page.
        page: PageId,
        /// The `request` the question carried.
        request: u64,
        /// The node under the point, or `None` when nothing is there.
        node: Option<Box<InspectedNode>>,
    },
    /// The answer to a [`PageSource::query_elements_in_rect`].
    ElementsInRect {
        /// The page.
        page: PageId,
        /// The `request` the question carried.
        request: u64,
        /// How many elements lie in the rect.
        count: usize,
    },
    /// The page's remote-debugging (CDP) target id is known. With
    /// [`PageSource::devtools_port`] it names the page's websocket:
    /// `ws://127.0.0.1:<port>/devtools/page/<id>`.
    DevtoolsTarget {
        /// The page.
        page: PageId,
        /// The CDP target id.
        id: String,
    },
}

/// The first message id a devtools client's message may carry through
/// [`PageSource::devtools_send`]. Ids below it are the backend's own
/// questions, so one id never names both.
pub const DEVTOOLS_CLIENT_ID_BASE: i32 = 1 << 30;

/// Where a page's devtools messages go: the answers to what
/// [`PageSource::devtools_send`] sent, and every event the page raises. The
/// text is one devtools-protocol JSON message. Called on the main thread,
/// from inside [`PageSource::pump`] or the backend's own message loop.
pub type DevtoolsSink = Arc<dyn Fn(PageId, &str) + Send + Sync>;

/// What a pointer did to an element: the two things interaction sync
/// replays (ADR 0030).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointKind {
    /// Moved onto it, or within it.
    Hover,
    /// Pressed and released the primary button on it.
    Click,
}

/// A move through a page's session history, or a new address for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageNav {
    /// Load this URL in the page, keeping its history.
    To(String),
    /// Go one entry back. Does nothing at the start of the history.
    Back,
    /// Go one entry forward. Does nothing at the end of the history.
    Forward,
    /// Load the current address again.
    Reload,
    /// Abandon the load in flight.
    Stop,
}

/// A DOM element a page found under a point: what a comment on it records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageElement {
    /// A CSS selector that finds the element in its document.
    pub selector: String,
    /// A readable path to the element, when the backend builds one.
    pub element_path: Option<String>,
    /// The element's box in the page's viewport, in CSS pixels.
    pub bounding_box: PixelRect,
}

/// A DOM node as the inspect tool reads it: what its outline, its popover
/// and the chat's pill show (`inspectionPayload` in the Electron app).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectedNode {
    /// What names the node within its page: its `id` attribute, else its
    /// `data-testid`, else `tag@left:top`.
    pub node_id: String,
    /// The tag name, in lower case.
    pub tag_name: String,
    /// The tag and the node's text in quotes (`button "Save"`), or the tag
    /// alone when it has no text, label, title, value, placeholder or alt.
    pub name: String,
    /// A CSS selector that finds the node in its document.
    pub selector: String,
    /// The `id` attribute, when it has one.
    pub id_attribute: Option<String>,
    /// The node's classes, in document order.
    pub classes: Vec<String>,
    /// Computed styles as `(property, value)`, in this order: `display`,
    /// `position`, `font-family`, `font-size`, `font-weight`, `color`,
    /// `background`, `padding`, `margin`. `background` is the computed
    /// background colour.
    pub styles: Vec<(String, String)>,
    /// The node's box in the page's viewport, in CSS pixels.
    pub bounding_box: PixelRect,
}

impl InspectedNode {
    /// The computed value of `property`, when the page reported it.
    pub fn style(&self, property: &str) -> Option<&str> {
        self.styles
            .iter()
            .find(|(name, _)| name == property)
            .map(|(_, value)| value.as_str())
    }
}

/// The `prefers-color-scheme` a page reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageColorScheme {
    /// `light`.
    Light,
    /// `dark`.
    Dark,
}

/// A backend that hosts offscreen pages and delivers their painted frames.
///
/// Object-safe on purpose: the app holds a `Box<dyn PageSource>` and picks the
/// CEF or synthetic backend at startup, and the bench drives either through
/// the same handle. Every method is called on the main thread (CEF on macOS
/// requires its UI thread to be the process main thread, shared with winit),
/// so implementors need not be `Send`.
pub trait PageSource {
    /// Short backend name for logs and bench reports (`"cef"`, `"synthetic"`).
    fn name(&self) -> &'static str;

    /// Starts hosting a page; frames for it arrive through
    /// [`drain_events`](Self::drain_events) after later [`pump`](Self::pump)s.
    fn create_page(&mut self, spec: &PageSpec) -> Result<PageId, PageSourceError>;

    /// Changes the page's CSS layout viewport (re-layout + full repaint).
    fn set_viewport(&mut self, page: PageId, viewport: CssSize) -> Result<(), PageSourceError>;

    /// Changes texels per CSS pixel (re-raster at the new density; layout is
    /// unchanged). Maps to CEF's device scale factor.
    fn set_texture_scale(&mut self, page: PageId, scale: f32) -> Result<(), PageSourceError>;

    /// Changes the target paint rate (frame-rate LOD).
    fn set_frame_rate(&mut self, page: PageId, fps: u32) -> Result<(), PageSourceError>;

    /// Painting policy: `false` stops painting (CEF `WasHidden(true)`) while
    /// keeping the page alive and its last frame valid.
    fn set_painting(&mut self, page: PageId, painting: bool) -> Result<(), PageSourceError>;

    /// Sets the `prefers-color-scheme` the page reports, and re-renders it
    /// for it (`applyPageColorScheme`). A backend whose pages have no such
    /// setting leaves them as they are.
    fn set_color_scheme(
        &mut self,
        _page: PageId,
        _scheme: PageColorScheme,
    ) -> Result<(), PageSourceError> {
        Ok(())
    }

    /// Stops hosting a page. Outstanding frames for it stay valid until dropped.
    fn close_page(&mut self, page: PageId) -> Result<(), PageSourceError>;

    /// Gives the page keyboard focus, or clears focus with `None`.
    fn set_focus(&mut self, page: Option<PageId>) -> Result<(), PageSourceError>;

    /// Forwards one input event into a page.
    fn send_input(&mut self, page: PageId, event: &InputEvent) -> Result<(), PageSourceError>;

    /// Does one slice of backend work without blocking (CEF
    /// `DoMessageLoopWork`). Call once per event-loop turn.
    fn pump(&mut self);

    /// Whether a page is waiting on [`pump`](Self::pump) to paint its next
    /// frame. A source whose pages paint on their own and queue what they
    /// painted says `false`, and its host may then sleep between events.
    fn paints_on_pump(&self) -> bool {
        false
    }

    /// Moves every pending event into `out` (appending; `out` is not cleared),
    /// so the caller can reuse one buffer across frames.
    fn drain_events(&mut self, out: &mut Vec<PageEvent>);

    /// Moves the page through its history or to a new address. What comes
    /// of it arrives as [`PageEvent::Url`], [`PageEvent::Title`] and
    /// [`PageEvent::Loading`].
    fn navigate(&mut self, page: PageId, nav: &PageNav) -> Result<(), PageSourceError>;

    /// Asks how far the page's document is scrolled, as a fraction of how
    /// far it can scroll. Answered by a [`PageEvent::ScrollProgress`].
    fn scroll_progress(&mut self, page: PageId) -> Result<(), PageSourceError>;

    /// Scrolls the page's document to `progress` of how far it can scroll
    /// along each axis, clamped to `0..=1`: the same fraction lands a short
    /// document and a long one at the same place in their content. The
    /// move is reported like any other, by a [`PageEvent::Scrolled`].
    fn scroll_to(&mut self, page: PageId, progress: Vec2) -> Result<(), PageSourceError>;

    /// Asks which element the page has under `point`, in its viewport CSS
    /// pixels. Every accepted question is answered once, by a
    /// [`PageEvent::ElementAt`] carrying `request`, unless the page closes
    /// or crashes first.
    fn query_element(
        &mut self,
        page: PageId,
        point: Vec2,
        request: u64,
    ) -> Result<(), PageSourceError>;

    /// Asks for the node the page has under `point`, in its viewport CSS
    /// pixels, as the inspect tool reads it. Answered like
    /// [`query_element`](Self::query_element), by a
    /// [`PageEvent::Inspected`].
    fn inspect_at(
        &mut self,
        page: PageId,
        point: Vec2,
        request: u64,
    ) -> Result<(), PageSourceError>;

    /// Asks how many elements the page has inside `rect`, in its viewport
    /// CSS pixels. Answered like [`query_element`](Self::query_element), by
    /// a [`PageEvent::ElementsInRect`].
    fn query_elements_in_rect(
        &mut self,
        page: PageId,
        rect: CssRect,
        request: u64,
    ) -> Result<(), PageSourceError>;

    /// Names the one page whose hovers and clicks are reported as
    /// [`PageEvent::Pointed`], or none. Only input the user gave is
    /// reported, so the page must be the one that receives it.
    fn set_capture(&mut self, page: Option<PageId>) -> Result<(), PageSourceError>;

    /// Asks the page for the elements `bundle` could be describing.
    /// Answered by a [`PageEvent::Candidates`] carrying `request`, unless
    /// the page closes or navigates first.
    fn query_candidates(
        &mut self,
        page: PageId,
        bundle: &LocatorBundle,
        request: u64,
    ) -> Result<(), PageSourceError>;

    /// Replays a hover or a click at `point`, in the page's viewport CSS
    /// pixels, as trusted input: the page reacts as it does to the user's
    /// (`:hover`, focus, native controls), whether or not it has focus.
    fn replay_pointer(
        &mut self,
        page: PageId,
        kind: PointKind,
        point: Vec2,
    ) -> Result<(), PageSourceError>;

    /// Port of the backend's remote-debugging (CDP) endpoint, if enabled.
    fn devtools_port(&self) -> Option<u16>;

    /// Sends one devtools-protocol message to the page's own devtools agent:
    /// a JSON object with an `id` of at least [`DEVTOOLS_CLIENT_ID_BASE`], a
    /// `method` and `params`. It speaks to this page and no other, however
    /// many are hosted. The answer, carrying the same `id`, goes to the
    /// [`DevtoolsSink`].
    fn devtools_send(&mut self, page: PageId, message: &str) -> Result<(), PageSourceError>;

    /// Sets where devtools answers and events go; `None` drops them.
    fn set_devtools_sink(&mut self, sink: Option<DevtoolsSink>);

    /// Closes every page and shuts the backend down, blocking until done.
    /// Idempotent.
    fn shutdown(&mut self);

    /// Non-blocking shutdown for a caller inside a running event loop: call
    /// once per loop turn until it returns `true`, then leave the loop. A
    /// backend that can only stop from the running loop (CEF on macOS)
    /// overrides this; the default shuts down at once.
    fn poll_shutdown(&mut self) -> bool {
        self.shutdown();
        true
    }
}
