//! Which comments the canvas shows: the open ones that are still about the
//! document they were made on.

use specular_doc::{Annotation, AnnotationStatus};

use crate::App;
use crate::anchor::matches_page_url;
use crate::app::page_of;

/// Whether `status` is a thread still being worked on.
pub(super) const fn is_open(status: AnnotationStatus) -> bool {
    match status {
        AnnotationStatus::Pending | AnnotationStatus::Acknowledged => true,
        AnnotationStatus::Resolved | AnnotationStatus::Dismissed => false,
    }
}

/// Whether `annotation` is drawn and can be hit: it is open and, when it is
/// bound to a page's document, that page is still there and still shows the
/// document the comment was made on, and the content it is on has not
/// scrolled out of the page. A comment left behind by a page that navigated
/// comes back with the page.
pub(crate) fn shown(app: &App, annotation: &Annotation) -> bool {
    if !is_open(annotation.status) {
        return false;
    }
    let Some(binding) = &annotation.page_anchor else {
        return true;
    };
    (app.document.entity(&binding.page_id))
        .and_then(page_of)
        .is_some_and(|page| matches_page_url(binding.page_url.as_deref(), Some(&page.url)))
        && !super::left_its_page(app, annotation)
}
