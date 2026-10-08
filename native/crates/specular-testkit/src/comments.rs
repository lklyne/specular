//! Comments: annotations to start a test from, and the shell's side of the
//! questions the app asks its pages about their elements.

use specular_core::{CapturedElement, PageElement, synthetic_capture};
use specular_doc::{Annotation, AnnotationAnchor, AnnotationId, Command, Document};
use specular_interact::{Effect, Event, PageGrab, PageNotice};

use crate::TestApp;

/// A pending comment `id` by the user at `anchor` reading `content`, made
/// at the start of 2026 and bound to no page.
pub fn comment(id: &str, anchor: AnnotationAnchor, content: &str) -> Annotation {
    let created_at = "2026-01-01T00:00:00.000Z".to_owned();
    Annotation {
        text: content.to_owned(),
        ..Annotation::new(AnnotationId::new(id), anchor, created_at)
    }
}

/// `document` with `annotation` after the ones it holds.
#[track_caller]
#[must_use]
pub fn with_comment(mut document: Document, annotation: Annotation) -> Document {
    let id = annotation.id.clone();
    let command = Command::InsertAnnotation {
        annotation: Box::new(annotation),
        at: document.annotations().len(),
    };
    if let Err(error) = document.apply(command) {
        panic!("annotation {id:?} could not be inserted: {error}");
    }
    document
}

impl TestApp {
    /// The comment being written, which is not in the document yet.
    #[track_caller]
    pub fn comment_draft(&self) -> &Annotation {
        match self.app().comment_draft() {
            Some(draft) => draft,
            None => panic!("no comment is being written"),
        }
    }

    /// Answers the latest [`Effect::QueryElement`] not yet drained, as the
    /// shell does: `element` is what the page has under the point asked
    /// about.
    #[track_caller]
    pub fn answer_element(&mut self, element: Option<PageElement>) -> &mut Self {
        let asked = self.effects().iter().rev().find_map(|effect| match effect {
            Effect::QueryElement { page, point } => Some((page.clone(), *point)),
            _ => None,
        });
        let Some((page, point)) = asked else {
            panic!("no page was asked for an element: {:?}", self.effects());
        };
        self.send(Event::ElementAt {
            page,
            point,
            element,
        })
    }

    /// The latest [`Effect::CaptureElement`] not yet drained: the page, the
    /// request and the document point asked about.
    #[track_caller]
    pub fn capture_asked(&self) -> (String, u64, glam::Vec2) {
        let asked = self.effects().iter().rev().find_map(|effect| match effect {
            Effect::CaptureElement {
                page,
                request,
                point,
            } => Some((page.as_str().to_owned(), *request, *point)),
            _ => None,
        });
        match asked {
            Some(asked) => asked,
            None => panic!("no page was asked what to attach to: {:?}", self.effects()),
        }
    }

    /// Answers the latest [`Effect::CaptureElement`] not yet drained with
    /// `element`, as the shell does.
    #[track_caller]
    pub fn answer_capture_with(&mut self, element: Option<CapturedElement>) -> &mut Self {
        let (page, request, _) = self.capture_asked();
        self.page_reports(&page, PageNotice::ElementCaptured { request, element })
    }

    /// Answers the latest [`Effect::CaptureElement`] not yet drained as a
    /// synthetic page does: with the cell of its 160x48 grid that holds the
    /// point.
    #[track_caller]
    pub fn answer_capture(&mut self) -> &mut Self {
        let (page, _, point) = self.capture_asked();
        let viewport = self.app().page_placement(&page.as_str().into());
        let Some(viewport) = viewport.map(|placement| placement.viewport) else {
            panic!("{page} is not a page");
        };
        self.answer_capture_with(Some(synthetic_capture(viewport, point)))
    }

    /// Answers the latest [`Effect::QueryRegionGrab`] not yet drained, as
    /// the shell does: `elements` is how many the region grabbed in each
    /// page asked about, in the order asked, and none in a page left out.
    #[track_caller]
    pub fn answer_grab(&mut self, elements: &[usize]) -> &mut Self {
        let asked = self.effects().iter().rev().find_map(|effect| match effect {
            Effect::QueryRegionGrab { region, pages } => Some((*region, pages.clone())),
            _ => None,
        });
        let Some((region, pages)) = asked else {
            panic!(
                "no page was asked what a region grabbed: {:?}",
                self.effects()
            );
        };
        let grabs = (pages.into_iter().enumerate())
            .map(|(index, covered)| PageGrab {
                page: covered.page,
                elements: elements.get(index).copied().unwrap_or(0),
            })
            .collect();
        self.send(Event::RegionGrab { region, grabs })
    }
}
