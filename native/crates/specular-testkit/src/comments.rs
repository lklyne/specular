//! Comments: annotations to start a test from, and the shell's side of the
//! two questions the comment tool asks it.

use specular_core::PageElement;
use specular_doc::{Annotation, AnnotationAnchor, AnnotationId, Command, Document};
use specular_interact::{Effect, Event, PageGrab};

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
