//! A backend page event as the notice the app is told.

use specular_core::{PageEvent, PageId};
use specular_interact::PageNotice;

/// What the app is told about `event`, with the backend page it is about.
/// `None` for frames, popups and answers, which go elsewhere.
#[expect(clippy::too_many_lines, reason = "one arm per kind of event")]
pub(crate) fn notice_of(
    event: &PageEvent,
    devtools_port: Option<u16>,
) -> Option<(PageId, PageNotice)> {
    let notice = match event {
        PageEvent::Loaded { page, http_status } => (
            *page,
            PageNotice::Loaded {
                http_status: *http_status,
            },
        ),
        PageEvent::Crashed { page, reason } => (
            *page,
            PageNotice::Crashed {
                reason: reason.clone(),
            },
        ),
        PageEvent::ImeCompositionBounds { page, bounds } => {
            (*page, PageNotice::ImeCompositionBounds(*bounds))
        }
        PageEvent::Title { page, title } => (*page, PageNotice::Title(title.clone())),
        PageEvent::Cursor { page, cursor } => (*page, PageNotice::Cursor(*cursor)),
        PageEvent::Favicon { page, png } => (*page, PageNotice::Favicon(png.clone())),
        PageEvent::Url { page, url } => (*page, PageNotice::Url(url.clone())),
        PageEvent::OpenTab {
            page,
            url,
            foreground,
        } => (
            *page,
            PageNotice::OpenTab {
                url: url.clone(),
                foreground: *foreground,
            },
        ),
        PageEvent::Loading {
            page,
            loading,
            can_go_back,
            can_go_forward,
        } => (
            *page,
            PageNotice::Loading {
                loading: *loading,
                can_go_back: *can_go_back,
                can_go_forward: *can_go_forward,
            },
        ),
        PageEvent::Scrolled { page, offset } => {
            let (x, y) = along(*offset);
            (*page, PageNotice::Scrolled { x, y })
        }
        PageEvent::ScrollProgress { page, progress } => {
            let (x, y) = along(*progress);
            (*page, PageNotice::ScrollProgress { x, y })
        }
        PageEvent::ScrollRoom { page, room } => (*page, PageNotice::ScrollRoom(*room)),
        PageEvent::Pointed { page, kind, bundle } => (
            *page,
            PageNotice::Pointed {
                kind: *kind,
                bundle: bundle.clone(),
            },
        ),
        PageEvent::Candidates {
            page,
            request,
            candidates,
        } => (
            *page,
            PageNotice::Candidates {
                request: *request,
                candidates: candidates.clone(),
            },
        ),
        PageEvent::ElementCaptured {
            page,
            request,
            element,
        } => (
            *page,
            PageNotice::ElementCaptured {
                request: *request,
                element: element.clone(),
            },
        ),
        PageEvent::ElementPlaces { page, places } => {
            (*page, PageNotice::ElementPlaces(places.clone()))
        }
        PageEvent::ViewportFit {
            page,
            covers,
            color,
        } => (
            *page,
            PageNotice::ViewportFit {
                covers: *covers,
                color: *color,
            },
        ),
        PageEvent::DevtoolsTarget { page, id } => {
            let port = devtools_port?;
            let url = format!("ws://127.0.0.1:{port}/devtools/page/{id}");
            (*page, PageNotice::DevtoolsUrl(url))
        }
        PageEvent::Frame(_)
        | PageEvent::FrameDropped { .. }
        | PageEvent::PopupVisibility { .. }
        | PageEvent::PopupRect { .. }
        | PageEvent::ElementAt { .. }
        | PageEvent::Inspected { .. }
        | PageEvent::ElementsInRect { .. } => return None,
    };
    Some(notice)
}

/// A vector's two parts as the notices carry them.
fn along(vector: glam::Vec2) -> (f64, f64) {
    (f64::from(vector.x), f64::from(vector.y))
}
