//! What an item view leaves out, and what it lets be made. Each answer
//! comes from [`others`], which the lens and the eye decide between them.

use glam::DVec2;
use specular_doc::{Annotation, AnnotationAnchor, Entity, EntityId};

use super::{ItemView, Lens, held};
use crate::anchor::anchors_to_pages;
use crate::app::page_of;
use crate::{App, Tool, geometry};

/// How much is seen beside the item shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Others {
    /// Nothing: the eye is shut.
    None,
    /// What is hooked to the item, and the comments on it. In Fill the item
    /// is the whole view, so what merely lies near it on the canvas has no
    /// place there.
    Hooked,
    /// Everything, where the canvas has it.
    All,
}

fn others(app: &App, view: &ItemView) -> Others {
    if app.session.others_hidden {
        return Others::None;
    }
    match app.session.tabs.lens(&view.item) {
        Lens::Fill => Others::Hooked,
        Lens::Device | Lens::Canvas => Others::All,
    }
}

/// The item view, when it leaves anything out, and how much it shows.
fn narrowed(app: &App) -> Option<(&ItemView, Others)> {
    let view = app.session.item_view.as_ref()?;
    let others = others(app, view);
    (others != Others::All).then_some((view, others))
}

fn hooked(view: &ItemView, entity: &Entity) -> bool {
    (entity.anchor.as_ref()).is_some_and(|anchor| anchor.page_id == view.item)
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
    others == Others::None || !hooked(view, entity)
}

/// Whether the view would hide `entity` as soon as it was added, hooked
/// where it can be: anything while the eye is shut or a Document fills the
/// view, and beside a page that fills it whatever cannot be hooked to one.
pub(crate) fn hides_new(app: &App, entity: &Entity) -> bool {
    let Some((view, others)) = narrowed(app) else {
        return false;
    };
    let page = (app.document.entity(&view.item)).is_some_and(|item| page_of(item).is_some());
    let hooks = page && entity.parent.is_none() && anchors_to_pages(&entity.kind);
    others == Others::None || !hooks
}

/// Whether a press at the canvas point `world` with `tool` makes nothing,
/// because the view would hide it as soon as it existed: anything while the
/// eye is shut, and in Fill what is made off the page, and any page or
/// Document.
pub(crate) fn refuses(app: &App, tool: Tool, world: DVec2) -> bool {
    let Some((view, others)) = narrowed(app) else {
        return false;
    };
    let hooks = match tool {
        Tool::AddText | Tool::AddSticky | Tool::AddShape | Tool::Draw | Tool::Comment => true,
        Tool::AddPage | Tool::AddDocument => false,
        Tool::Select | Tool::Inspect => return false,
    };
    let on_page = (app.page_placement(&view.item))
        .is_some_and(|placement| geometry::contains(placement.rect, world));
    others == Others::None || !(hooks && on_page)
}

/// The page that is the only one there is to hook to: the item shown,
/// while only what is hooked to it is seen.
pub(crate) fn only_page(app: &App) -> Option<&EntityId> {
    narrowed(app)
        .filter(|(_, others)| *others == Others::Hooked)
        .map(|(view, _)| &view.item)
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

/// Whether the view leaves `annotation` out: every comment while the eye
/// is shut, and in Fill one that is not on the page shown.
pub(crate) fn hides_comment(app: &App, annotation: &Annotation) -> bool {
    let Some((view, others)) = narrowed(app) else {
        return false;
    };
    others == Others::None || comment_page(annotation) != Some(&view.item)
}

/// Whether `entity` cannot be brought into view where the view is: it is
/// left out, or the camera is held on an item it is neither part of nor
/// hooked to.
pub(crate) fn out_of_reach(app: &App, entity: &Entity) -> bool {
    let away = |view: &ItemView| entity.id != view.item && !hooked(view, entity);
    hides(app, entity) || held(app).is_some_and(away)
}

/// Whether `annotation` cannot be brought into view where the view is.
pub(crate) fn comment_out_of_reach(app: &App, annotation: &Annotation) -> bool {
    let away = |view: &ItemView| comment_page(annotation) != Some(&view.item);
    hides_comment(app, annotation) || held(app).is_some_and(away)
}
