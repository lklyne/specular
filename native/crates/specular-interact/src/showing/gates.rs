//! What an item view leaves out, and what it lets be made. Each answer
//! comes from [`narrowed`], which the lens and the eye decide between them.

use glam::DVec2;
use specular_doc::{Annotation, AnnotationAnchor, Entity, EntityId};

use super::{ItemView, Lens};
use crate::anchor::anchors_to_pages;
use crate::app::is_page;
use crate::{App, Tool, geometry};

/// How much is seen beside the item shown, when it is not everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Others {
    /// What follows an element of the item. In Fill the item is the whole
    /// view, and a page is laid out there at another width than the one it
    /// is stored at. What merely lies near it on the canvas has no place,
    /// and neither has what is hooked to it by a position or a scroll, which
    /// was placed against the stored layout.
    Following,
    /// What is hooked to the item, however: Device with the eye shut. The
    /// item is laid out as it is stored, so all of it is where it was put.
    Hooked,
}

/// The item view, when it leaves anything out, and how much it shows.
/// Device with the eye open sees everything, where the canvas has it.
fn narrowed(app: &App) -> Option<(&ItemView, Others)> {
    let view = app.session.item_view.as_ref()?;
    match app.session.tabs.lens(&view.item) {
        Lens::Fill => Some((view, Others::Following)),
        Lens::Device => (!app.session.others_shown).then_some((view, Others::Hooked)),
    }
}

fn hooked(view: &ItemView, entity: &Entity) -> bool {
    (entity.anchor.as_ref()).is_some_and(|anchor| anchor.page_id == view.item)
}

/// Whether `entity` is seen beside the item shown: it is hooked to it, and
/// where only what follows is seen it follows an element or has yet to hear
/// which one. Its page is asked after the step that places it.
fn kept(app: &App, view: &ItemView, others: Others, entity: &Entity) -> bool {
    let element = (entity.anchor.as_ref()).is_some_and(|anchor| anchor.element.is_some());
    let follows = element || app.session.attach.awaits(entity);
    hooked(view, entity) && (others == Others::Hooked || follows)
}

/// Whether the view leaves `entity` out. The item shown never is, nor what
/// a drag is making, which is hooked when the drag ends.
pub(crate) fn hides(app: &App, entity: &Entity) -> bool {
    let Some((view, others)) = narrowed(app) else {
        return false;
    };
    if entity.id == view.item || app.creating() == Some(&entity.id) {
        return false;
    }
    !kept(app, view, others, entity)
}

/// Whether the view would hide `entity` as soon as it was added, hooked
/// where it can be: anything beside a Document, and beside a page whatever
/// cannot be hooked to one.
pub(crate) fn hides_new(app: &App, entity: &Entity) -> bool {
    let Some((view, _)) = narrowed(app) else {
        return false;
    };
    let page = (app.document.entity(&view.item)).is_some_and(is_page);
    let hooks = page && entity.parent.is_none() && anchors_to_pages(&entity.kind);
    !hooks
}

/// Whether a press at the canvas point `world` with `tool` makes nothing,
/// because the view would hide it as soon as it existed: what is made off
/// the page, and any page or Document.
pub(crate) fn refuses(app: &App, tool: Tool, world: DVec2) -> bool {
    let Some((view, _)) = narrowed(app) else {
        return false;
    };
    let hooks = match tool {
        Tool::AddText | Tool::AddSticky | Tool::AddShape | Tool::Draw | Tool::Comment => true,
        Tool::AddPage | Tool::AddDocument => false,
        Tool::Select | Tool::Inspect => return false,
    };
    let on_page = (app.page_placement(&view.item))
        .is_some_and(|placement| geometry::contains(placement.rect, world));
    !(hooks && on_page)
}

/// The page that is the only one there is to hook to: the item shown,
/// while only what is hooked to it is seen.
pub(crate) fn only_page(app: &App) -> Option<&EntityId> {
    narrowed(app).map(|(view, _)| &view.item)
}

/// The page `annotation` is on, if it is on one.
fn comment_page(annotation: &Annotation) -> Option<&EntityId> {
    match &annotation.anchor {
        AnnotationAnchor::Page { page_id, .. } | AnnotationAnchor::Element { page_id, .. } => {
            Some(page_id)
        }
        AnnotationAnchor::Canvas { .. } | AnnotationAnchor::Region(_) => {
            annotation.page_anchor.as_ref().map(|it| &it.page_id)
        }
    }
}

/// Whether `annotation` is on an element of its page: a comment on an
/// element, or a region that follows one or has yet to hear which.
fn comment_follows(app: &App, annotation: &Annotation) -> bool {
    match &annotation.anchor {
        AnnotationAnchor::Element { .. } => true,
        AnnotationAnchor::Region(_) => {
            let element = (annotation.page_anchor.as_ref()).is_some_and(|it| it.element.is_some());
            element || app.session.attach.awaits_comment(annotation)
        }
        AnnotationAnchor::Canvas { .. } | AnnotationAnchor::Page { .. } => false,
    }
}

/// Whether the view leaves `annotation` out: one that is not on the page
/// shown, and in Fill one that is not on an element of it.
pub(crate) fn hides_comment(app: &App, annotation: &Annotation) -> bool {
    let Some((view, others)) = narrowed(app) else {
        return false;
    };
    let on_item = comment_page(annotation) == Some(&view.item);
    !(on_item && (others == Others::Hooked || comment_follows(app, annotation)))
}

/// Whether `entity` cannot be brought into view where the view is: it is
/// left out, or the camera is held on an item it is neither part of nor
/// hooked to.
pub(crate) fn out_of_reach(app: &App, entity: &Entity) -> bool {
    let away = |view: &ItemView| entity.id != view.item && !hooked(view, entity);
    hides(app, entity) || app.session.item_view.as_ref().is_some_and(away)
}

/// Whether `annotation` cannot be brought into view where the view is.
pub(crate) fn comment_out_of_reach(app: &App, annotation: &Annotation) -> bool {
    let away = |view: &ItemView| comment_page(annotation) != Some(&view.item);
    hides_comment(app, annotation) || app.session.item_view.as_ref().is_some_and(away)
}
