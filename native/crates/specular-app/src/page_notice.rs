//! A backend page event as the notice the app is told.

use specular_core::{PageEvent, PageId};
use specular_interact::PageNotice;

/// What the app is told about `event`, with the backend page it is about.
/// `None` for frames, popups and answers, which go elsewhere.
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
        PageEvent::Favicon { page, png } => (*page, PageNotice::Favicon(png.clone())),
        PageEvent::Url { page, url } => (*page, PageNotice::Url(url.clone())),
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
        PageEvent::Scrolled { page, offset } => (
            *page,
            PageNotice::Scrolled {
                x: f64::from(offset.x),
                y: f64::from(offset.y),
            },
        ),
        PageEvent::ScrollProgress { page, progress } => (
            *page,
            PageNotice::ScrollProgress {
                x: f64::from(progress.x),
                y: f64::from(progress.y),
            },
        ),
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

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use super::*;

    const PAGE: PageId = PageId(4);

    #[test]
    fn a_devtools_target_becomes_the_pages_websocket_on_the_debugging_port() {
        let event = PageEvent::DevtoolsTarget {
            page: PAGE,
            id: "ABC123".to_owned(),
        };
        let url = "ws://127.0.0.1:9222/devtools/page/ABC123".to_owned();
        assert_eq!(
            notice_of(&event, Some(9222)),
            Some((PAGE, PageNotice::DevtoolsUrl(url)))
        );
        assert_eq!(notice_of(&event, None), None);
    }

    #[test]
    fn a_scroll_is_a_notice_and_an_answer_is_not() {
        let event = PageEvent::Scrolled {
            page: PAGE,
            offset: Vec2::new(0.0, 120.5),
        };
        assert_eq!(
            notice_of(&event, None),
            Some((PAGE, PageNotice::Scrolled { x: 0.0, y: 120.5 }))
        );
        let answer = PageEvent::ElementsInRect {
            page: PAGE,
            request: 1,
            count: 2,
        };
        assert_eq!(notice_of(&answer, Some(9222)), None);
    }
}
