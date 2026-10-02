//! The page backend contract.

use crate::frame::FrameEvent;
use crate::geometry::{CssSize, PixelRect};
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

    /// Moves every pending event into `out` (appending; `out` is not cleared),
    /// so the caller can reuse one buffer across frames.
    fn drain_events(&mut self, out: &mut Vec<PageEvent>);

    /// Port of the backend's remote-debugging (CDP) endpoint, if enabled.
    fn devtools_port(&self) -> Option<u16>;

    /// Closes every page and shuts the backend down. Idempotent.
    fn shutdown(&mut self);
}
