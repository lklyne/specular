//! The page backend contract.

mod device;
mod element;
mod event;

pub use self::device::{PageColorScheme, PageDevice, PageSafeArea, PageUserAgent};
pub use self::element::{CapturedElement, ElementPlace, InspectedNode, PageElement};
pub use self::event::PageEvent;

use std::sync::Arc;

use glam::Vec2;

use crate::geometry::{CssRect, CssSize};
use crate::input::InputEvent;
use crate::locator::LocatorBundle;
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

/// A backend that hosts offscreen pages and delivers their painted frames.
///
/// Object-safe on purpose: the app holds a `Box<dyn PageSource>` and picks the
/// CEF or synthetic backend at startup, and the bench drives either through
/// the same handle. Every method is called on the main thread (CEF on macOS
/// requires its UI thread to be the process main thread, shared with the window),
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

    /// Sends the pages every viewport and texture scale change made since
    /// the last call, each page's as one resize. Call when a loop turn has
    /// made all its changes: a backend that resizes one step at a time
    /// would otherwise paint a new viewport at the old scale first. A
    /// backend that applies each change as it is made has nothing to do.
    fn flush_geometry(&mut self) {}

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

    /// Sets the device the page tells its document it is on. The document
    /// loaded keeps the user agent it was fetched with, so a caller that
    /// changes one reloads the page. A backend whose pages are no device
    /// leaves them as they are.
    fn set_device(&mut self, _page: PageId, _device: &PageDevice) -> Result<(), PageSourceError> {
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

    /// Asks whether anything under `at`, in the page's CSS pixels, can
    /// scroll with a finger that moves `toward`. Answered by a
    /// [`PageEvent::ScrollRoom`]. A source that cannot tell does not answer.
    fn scroll_room(
        &mut self,
        _page: PageId,
        _at: Vec2,
        _toward: Vec2,
    ) -> Result<(), PageSourceError> {
        Ok(())
    }

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

    /// Asks how many elements a region at `rect` grabs, in the page's
    /// viewport CSS pixels, and for the boxes of the elements inside it. Answered like [`query_element`](Self::query_element), by
    /// a [`PageEvent::ElementsInRect`].
    fn query_elements_in_rect(
        &mut self,
        page: PageId,
        rect: CssRect,
        request: u64,
    ) -> Result<(), PageSourceError>;

    /// Asks which element an item centred on `point`, in the page's document
    /// CSS pixels, should follow: the nearest meaningful element under the
    /// point, else the nearest at that height, else the body. Answered by a
    /// [`PageEvent::ElementCaptured`] carrying `request`, unless the page
    /// closes or crashes first.
    fn capture_element(
        &mut self,
        page: PageId,
        point: Vec2,
        request: u64,
    ) -> Result<(), PageSourceError>;

    /// Names the elements whose place in the document the page reports from
    /// now on, as [`PageEvent::ElementPlaces`]: all of them once, and then
    /// each when it moves. Replaces the set named before; an empty set
    /// stops the reports.
    fn track_elements(&mut self, page: PageId, selectors: &[String])
    -> Result<(), PageSourceError>;

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
