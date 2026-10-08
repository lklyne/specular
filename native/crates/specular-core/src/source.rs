//! The page backend contract.

use glam::Vec2;

use crate::frame::FrameEvent;
use crate::geometry::{CssRect, CssSize, PixelRect};
use crate::input::InputEvent;
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
    /// The answer to a [`PageSource::query_element`].
    ElementAt {
        /// The page.
        page: PageId,
        /// The `request` the question carried.
        request: u64,
        /// The element under the point, or `None` when nothing is there.
        element: Option<PageElement>,
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

    /// Asks how many elements the page has inside `rect`, in its viewport
    /// CSS pixels. Answered like [`query_element`](Self::query_element), by
    /// a [`PageEvent::ElementsInRect`].
    fn query_elements_in_rect(
        &mut self,
        page: PageId,
        rect: CssRect,
        request: u64,
    ) -> Result<(), PageSourceError>;

    /// Port of the backend's remote-debugging (CDP) endpoint, if enabled.
    fn devtools_port(&self) -> Option<u16>;

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
