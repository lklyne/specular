//! What a hosted page reports.

use glam::Vec2;
use specular_doc::EntityId;

use crate::{App, Effect, PageNotice, comment, page_state, sync};

/// A hosted page reported something.
pub(super) fn on_page_event(
    app: &mut App,
    page: &EntityId,
    notice: &PageNotice,
    effects: &mut Vec<Effect>,
) {
    let shown = app.page_state(page).and_then(|state| state.url.clone());
    let fitted = crate::safe_area::fitted(app, page);
    // The address is the one thing a page reports that is saved.
    if page_state::on_notice(app, page, notice) {
        effects.push(Effect::Save);
    }
    crate::safe_area::follow_fit(fitted, app, page, effects);
    sync::on_notice(app, page, notice, shown.as_deref(), effects);
    crate::inspect::on_notice(app, page, notice);
    match notice {
        PageNotice::Scrolled { .. } => crate::flick::scrolled(app, page),
        PageNotice::ScrollRoom(room) => crate::flick::room(app, page, *room),
        PageNotice::RegionElements(boxes) => comment::on_elements(app, page, boxes),
        PageNotice::OpenTab { url, foreground } => {
            crate::link_open::open(app, page, url, *foreground, effects);
        }
        _ => {}
    }
    on_page_notice(app, page, notice, effects);
}

fn on_page_notice(app: &App, page: &EntityId, notice: &PageNotice, effects: &mut Vec<Effect>) {
    match notice {
        PageNotice::Loaded { .. }
        | PageNotice::Crashed { .. }
        | PageNotice::Title(_)
        | PageNotice::Cursor(_)
        | PageNotice::Favicon(_)
        | PageNotice::Url(_)
        | PageNotice::Loading { .. }
        | PageNotice::Scrolled { .. }
        | PageNotice::ScrollProgress { .. }
        | PageNotice::ScrollRoom(_)
        | PageNotice::Pointed { .. }
        | PageNotice::Candidates { .. }
        | PageNotice::Inspected { .. }
        | PageNotice::RegionElements(_)
        | PageNotice::ElementCaptured { .. }
        | PageNotice::ElementPlaces(_)
        | PageNotice::DevtoolsUrl(_)
        | PageNotice::OpenTab { .. }
        | PageNotice::ViewportFit { .. } => {}
        PageNotice::ImeCompositionBounds(bounds) => {
            if app.session.focus.page() == Some(page)
                && let Some(bounds) = bounds
                && let Some(placement) = app.page_placement(page)
            {
                let camera = &app.session.camera;
                let per_css = placement.canvas_per_css().as_vec2();
                let corner = Vec2::new(bounds.x as f32, bounds.y as f32);
                let world = placement.rect_origin() + corner * per_css;
                effects.push(Effect::SetImeCursorArea {
                    origin: camera.world_to_screen(world),
                    size: Vec2::new(bounds.width as f32, bounds.height as f32)
                        * per_css
                        * camera.zoom,
                });
            }
        }
    }
}
