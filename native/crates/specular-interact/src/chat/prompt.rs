//! What the agent's prompt needs from the app: where to work, what the turn
//! is about, and where each comment of the thread sits.

use std::fmt::Write as _;

use serde_json::Value;
use specular_agent::{CommentContext, Pill, PromptContext, ThreadId, WriteTarget, origin_of};
use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationId, EntityId, Kind, Rect, RegionAnchor,
};

use crate::App;

/// Where a turn aimed at `pill` may change source: the repo bound to the
/// pill's origin, else the space folder.
///
/// The origin is the DOM node's for a DOM pill and the page-bound comment's
/// for a comment pill (its recorded address, else the page's live address,
/// else the page entity's). This is deliberately narrower than Electron's
/// `resolveWriteTarget`, which also resolves a selected page: here a canvas
/// selection, the empty pill, a canvas-bound comment and a `data:` page all
/// write to the space.
pub(crate) fn write_target(app: &App, pill: &Pill) -> WriteTarget {
    let origin = match pill {
        Pill::Dom { origin, .. } => origin.clone(),
        Pill::Annotation { annotation_id, .. } => app
            .document
            .annotation(&AnnotationId::new(annotation_id.as_str()))
            .and_then(|annotation| comment_origin(app, annotation)),
        Pill::Selection { .. } | Pill::Empty => None,
    };
    let bound = origin
        .as_deref()
        .and_then(|origin| Some((origin, app.repos.binding(origin)?)));
    match bound {
        Some((origin, binding)) => WriteTarget::Repo {
            origin: origin_of(origin).unwrap_or_else(|| origin.to_owned()),
            repo_path: binding.repo_path.to_owned(),
        },
        None => WriteTarget::Space,
    }
}

/// The origin of the page a comment is bound to, or `None` for a comment
/// on the canvas.
pub(crate) fn comment_origin(app: &App, annotation: &Annotation) -> Option<String> {
    let anchor = annotation.page_anchor.as_ref()?;
    let recorded = anchor.page_url.as_deref().and_then(origin_of);
    recorded.or_else(|| {
        let live = app
            .page_state(&anchor.page_id)
            .and_then(|s| s.url.as_deref());
        let entity = match &app.document.entity(&anchor.page_id)?.kind {
            Kind::Page(page) => Some(page.url.as_str()),
            Kind::Text(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => {
                None
            }
        };
        live.and_then(origin_of)
            .or_else(|| entity.and_then(origin_of))
    })
}

/// The prompt context for a turn on `thread` aimed at `pill`.
pub(super) fn context(app: &App, thread: &ThreadId, pill: Pill) -> PromptContext {
    let comments = app
        .threads
        .get(thread)
        .map(|thread| {
            thread
                .annotation_ids
                .iter()
                .filter_map(|id| app.document.annotation(&AnnotationId::new(id.as_str())))
                .map(|annotation| CommentContext {
                    annotation_id: annotation.id.as_str().to_owned(),
                    description: describe(app, annotation),
                })
                .collect()
        })
        .unwrap_or_default();
    PromptContext {
        space_path: app.space.folder().unwrap_or_default().to_owned(),
        write_target: write_target(app, &pill),
        pill,
        canvas_name: app.space.active().name.clone(),
        comments,
    }
}

/// One line saying what a comment is pinned to.
fn describe(app: &App, annotation: &Annotation) -> String {
    let page_of_anchor = || annotation.page_anchor.as_ref().map(|a| &a.page_id);
    let mut line = match &annotation.anchor {
        AnnotationAnchor::Element {
            page_id, selector, ..
        } => format!(
            "on element \"{selector}\" of {}",
            page_ref(app, annotation, page_id)
        ),
        AnnotationAnchor::Canvas { canvas_x, canvas_y } => {
            format!(
                "at canvas point ({}, {})",
                whole(*canvas_x),
                whole(*canvas_y)
            )
        }
        AnnotationAnchor::Page {
            page_id,
            offset_x,
            offset_y,
        } => format!(
            "at ({}, {}) on {}",
            whole(*offset_x),
            whole(*offset_y),
            page_ref(app, annotation, page_id)
        ),
        AnnotationAnchor::Region(RegionAnchor::Canvas { canvas_rect }) => {
            format!("on region {} of the canvas", region(*canvas_rect))
        }
        AnnotationAnchor::Region(RegionAnchor::Document { doc_rect }) => match page_of_anchor() {
            Some(page) => format!(
                "on region {} of {}",
                region(*doc_rect),
                page_ref(app, annotation, page)
            ),
            None => format!("on region {} of a page", region(*doc_rect)),
        },
    };
    if let Some(Value::Array(ids)) = annotation
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("selectionEntityIds"))
    {
        let ids: Vec<&str> = ids.iter().filter_map(Value::as_str).collect();
        if !ids.is_empty() {
            let _ = write!(line, "; the selection was {}", ids.join(", "));
        }
    }
    line
}

/// `page p1 (https://example.com/)`: the page's address as it is now, else
/// as the comment recorded it.
fn page_ref(app: &App, annotation: &Annotation, page: &EntityId) -> String {
    let live = app
        .document
        .entity(page)
        .and_then(|entity| match &entity.kind {
            Kind::Page(page) => Some(page.url.as_str()),
            Kind::Text(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => {
                None
            }
        });
    let recorded = annotation
        .page_anchor
        .as_ref()
        .and_then(|a| a.page_url.as_deref());
    match live.or(recorded).filter(|url| !url.is_empty()) {
        Some(url) => format!("page {page} ({url})"),
        None => format!("page {page}"),
    }
}

fn whole(value: f64) -> i64 {
    value.round() as i64
}

fn region(rect: Rect) -> String {
    format!(
        "({}, {}, {}\u{d7}{})",
        whole(rect.x),
        whole(rect.y),
        whole(rect.width),
        whole(rect.height)
    )
}
