//! What a page source reports: frames, loads, navigation and the answers to questions.

use std::sync::Arc;

use glam::Vec2;

use super::PointKind;
use super::element::{CapturedElement, ElementPlace, InspectedNode, PageElement};
use crate::cursor::Cursor;
use crate::frame::FrameEvent;
use crate::geometry::{CssRect, PixelRect};
use crate::locator::{LocatorBundle, LocatorCandidate};
use crate::page::PageId;

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
    /// The document's icon changed (CEF `OnFaviconURLChange`).
    Favicon {
        /// The page.
        page: PageId,
        /// The icon as a PNG; `None` when the document has none, or it
        /// could not be fetched.
        png: Option<Arc<[u8]>>,
    },
    /// The main frame shows another address (CEF `OnAddressChange`): a
    /// navigation committed, or the page changed its own URL in place.
    Url {
        /// The page.
        page: PageId,
        /// The address now shown.
        url: String,
    },
    /// The page asked for a link to open in a new tab (CEF
    /// `OnBeforePopup` or `OnOpenURLFromTab`). The backend refused it the
    /// window; the app decides whether a page opens instead.
    OpenTab {
        /// The page.
        page: PageId,
        /// The address.
        url: String,
        /// Whether the tab would have come to the front.
        foreground: bool,
    },
    /// What is under the pointer asks for another cursor (CEF
    /// `OnCursorChange`).
    Cursor {
        /// The page.
        page: PageId,
        /// The cursor asked for.
        cursor: Cursor,
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
    /// The answer to a [`PageSource::scroll_room`].
    ScrollRoom {
        /// The page.
        page: PageId,
        /// Whether something under the point can scroll the way asked.
        room: bool,
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
        /// How many elements a region there grabs.
        count: usize,
        /// The box of each visible element that lies wholly in the rect,
        /// in the page's viewport CSS pixels and document order.
        boxes: Vec<CssRect>,
    },
    /// The answer to a [`PageSource::capture_element`].
    ElementCaptured {
        /// The page.
        page: PageId,
        /// The `request` the question carried.
        request: u64,
        /// The element to attach to, or `None` for a document with no body
        /// and for an answer that could not be read.
        element: Option<CapturedElement>,
    },
    /// Where the elements a page is tracking ([`PageSource::track_elements`])
    /// now sit in its document. Only the ones that moved, appeared or went
    /// are listed.
    ElementPlaces {
        /// The page.
        page: PageId,
        /// Each selector with where its element is, or `None` when the
        /// selector finds nothing.
        places: Vec<(String, Option<ElementPlace>)>,
    },
    /// What the document says of the screen around it, told after it has
    /// been parsed and again once it has loaded.
    ViewportFit {
        /// The page.
        page: PageId,
        /// Whether its viewport meta asks for `viewport-fit=cover`: the
        /// whole screen, under whatever the device puts over it.
        covers: bool,
        /// The colour a browser paints round it, as red, green and blue:
        /// its `theme-color`, else its background. `None` when it names
        /// neither.
        color: Option<[u8; 3]>,
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
