//! What a page says about itself.

use std::sync::Arc;

use glam::Vec2;
use specular_core::{
    CapturedElement, Cursor, ElementPlace, InspectedNode, LocatorBundle, LocatorCandidate,
    PixelRect, PointKind,
};

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
    /// The document's icon changed: a PNG, or `None` for a document with
    /// none.
    Favicon(Option<Arc<[u8]>>),
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
    /// Whether anything under the page's finger can scroll the way it is
    /// moving: the answer to an
    /// [`Effect::AskScrollRoom`](crate::Effect::AskScrollRoom).
    ScrollRoom(bool),
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
    /// The boxes of the elements the page has inside the comment region
    /// being dragged, in its viewport CSS pixels: the answer to an
    /// [`Effect::PreviewRegion`](crate::Effect::PreviewRegion).
    RegionElements(Vec<specular_core::CssRect>),
    /// What is under the pointer in the page asks for this cursor.
    Cursor(Cursor),
    /// The page's remote-debugging websocket is known.
    DevtoolsUrl(String),
    /// The page asked for a link to open in a new tab: `target=_blank`,
    /// `window.open`, a Command-click or a middle click. The page was not
    /// let open it itself.
    OpenTab {
        /// The address.
        url: String,
        /// Whether the tab would have come to the front: a plain click on
        /// a `target=_blank` link or `window.open`, not a Command-click or
        /// a middle click.
        foreground: bool,
    },
    /// What the document says of the screen around it.
    ViewportFit {
        /// Whether it asks for the whole screen (`viewport-fit=cover`),
        /// under whatever its device puts over it.
        covers: bool,
        /// The colour a browser paints round it, as red, green and blue.
        /// `None` when it names none.
        color: Option<[u8; 3]>,
    },
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
