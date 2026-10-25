//! Comments: annotations to start a test from, and the shell's side of the
//! questions the app asks its pages about their elements.

use specular_doc::{Annotation, AnnotationAnchor, AnnotationId, Command, Document};

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
