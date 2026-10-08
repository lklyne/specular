//! What the composer's chip says about the comment draft.

use specular_doc::{Annotation, AnnotationAnchor};

use super::model::DraftKind;

/// The kind of `draft` and the words on its chip: the gesture that made it,
/// as `CommentDraft` names them in the Electron app.
pub(crate) fn describe(draft: &Annotation) -> (DraftKind, String) {
    match &draft.anchor {
        AnnotationAnchor::Element {
            selector,
            element_path,
            ..
        } => {
            let named = [Some(selector.as_str()), element_path.as_deref()]
                .into_iter()
                .flatten()
                .map(str::trim)
                .find(|name| !name.is_empty());
            (DraftKind::Element, named.unwrap_or("Element").to_owned())
        }
        AnnotationAnchor::Region(_) => match selected_count(draft) {
            Some(1) => (DraftKind::Selection, "1 item".to_owned()),
            Some(count) => (DraftKind::Selection, format!("{count} items")),
            None => (DraftKind::Region, "Area".to_owned()),
        },
        // A draft is never on a page point; it reads as the canvas point.
        AnnotationAnchor::Canvas { .. } | AnnotationAnchor::Page { .. } => {
            (DraftKind::Point, "Canvas point".to_owned())
        }
    }
}

/// How many entities a selection draft is about, or `None` for a draft that
/// is not one.
fn selected_count(draft: &Annotation) -> Option<usize> {
    let ids = draft.metadata.as_ref()?.get("selectionEntityIds")?;
    ids.as_array().map(Vec::len)
}
