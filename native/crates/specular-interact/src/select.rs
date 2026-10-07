//! What a left press does with the select tool.
//!
//! Pages are select-first, interact-second (ADR 0022). The first press on a
//! page selects it. A press on the page that is already the whole selection
//! *enters* it: it takes keyboard focus, and from then on presses on its
//! body go to the page, modifiers and all. The entering press itself is not
//! forwarded. Escape, or selecting anything else, leaves the page.

use glam::DVec2;
use specular_core::Modifiers;
use specular_doc::{EntityId, ItemId, Kind};

use crate::focus::set_focus;
use crate::marquee::MarqueeMode;
use crate::{App, Effect, Gesture, Handle, HandleOwner, Hit, PointerInput, caps, hit};

/// Whether a click with these modifiers changes the selection item by item
/// instead of replacing it.
pub(crate) const fn is_additive(modifiers: Modifiers) -> bool {
    modifiers.shift || modifiers.meta || modifiers.control
}

/// Offers a left press to the select tool. Returns `false` only when the
/// press is on the body of the entered page, which gets it instead.
pub(crate) fn press(
    app: &mut App,
    input: &PointerInput,
    click_count: u8,
    effects: &mut Vec<Effect>,
) -> bool {
    let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
    let hit = match hit::hit_test(app, input.screen) {
        // Edges cannot be drawn yet, so an anchor passes the press to what
        // is under it.
        Hit::Anchor { .. } => hit::body_at(app, input.screen),
        other => other,
    };
    match hit {
        Hit::PageContent { page, .. } if app.session.focus.page() == Some(&page) => return false,
        Hit::PageContent { page, .. } => press_page(app, page, world, input, click_count, effects),
        Hit::Handle { owner, handle } => press_handle(app, &owner, handle, world),
        Hit::GroupLabel { group } | Hit::GroupBorder { group } => select_unless_held(app, group),
        Hit::EntityBody { entity } => press_body(app, entity, world, input),
        Hit::Edge { edge } => {
            let item = ItemId::Edge(edge);
            if is_additive(input.modifiers) {
                app.session.selection.toggle(item);
            } else {
                app.session.selection.set([item]);
            }
        }
        Hit::Empty => begin_marquee(app, None, world, input),
        // `body_at` returns bodies only.
        Hit::Anchor { .. } => {}
    }
    true
}

fn press_page(
    app: &mut App,
    page: EntityId,
    world: DVec2,
    input: &PointerInput,
    click_count: u8,
    effects: &mut Vec<Effect>,
) {
    let modifiers = input.modifiers;
    if modifiers.meta || modifiers.control {
        begin_marquee(app, Some(page), world, input);
    } else if modifiers.shift {
        app.session.selection.toggle(ItemId::Entity(page));
    } else if modifiers.alt {
        let Some(start) = app.document.entity(&page).map(|entity| entity.rect) else {
            return;
        };
        select_unless_held(app, page.clone());
        app.session.gesture = Some(Gesture::Move {
            origin: world,
            items: vec![(page, start)],
        });
    } else if click_count > 1 || app.session.selection.single_entity() == Some(&page) {
        // The second click of a double-click enters however fast the two
        // landed, and whatever the first one found selected.
        app.session.selection.set([ItemId::Entity(page.clone())]);
        set_focus(app, Some(page), effects);
    } else {
        select_unless_held(app, page);
    }
}

fn press_body(app: &mut App, entity: EntityId, world: DVec2, input: &PointerInput) {
    let modifiers = input.modifiers;
    let item = ItemId::Entity(entity.clone());
    if modifiers.meta || modifiers.control {
        begin_marquee(app, Some(entity), world, input);
    } else if modifiers.shift {
        app.session.selection.toggle(item);
    } else if is_group(app, &entity) && !app.session.selection.contains(&item) {
        // A group's interior is canvas until the group is selected: a drag
        // marquees what is inside, and a click selects the group.
        begin_marquee(app, Some(entity), world, input);
    } else {
        select_unless_held(app, entity);
    }
}

/// A handle press resizes a page from a corner. Every other handle holds the
/// press until its resize exists.
fn press_handle(app: &mut App, owner: &HandleOwner, handle: Handle, world: DVec2) {
    let (HandleOwner::Entity(id), Handle::Corner(corner)) = (owner, handle) else {
        return;
    };
    let Some(entity) = app.document.entity(id) else {
        return;
    };
    match &entity.kind {
        Kind::Page(_) => {
            app.session.gesture = Some(Gesture::Resize {
                entity: id.clone(),
                corner,
                grab: corner.point(entity.rect) - world,
                start: entity.rect,
                min_size: caps::min_size(&entity.kind),
            });
        }
        Kind::Text(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => {}
    }
}

/// Selects `entity` alone, unless the press is on the selection already (a
/// member, or anything that moves with one), which it then keeps.
fn select_unless_held(app: &mut App, entity: EntityId) {
    if !app.selection_scope().holds(&entity) {
        app.session.selection.set([ItemId::Entity(entity)]);
    }
}

fn begin_marquee(app: &mut App, origin: Option<EntityId>, world: DVec2, input: &PointerInput) {
    app.session.gesture = Some(Gesture::Marquee {
        start: world,
        start_screen: input.screen,
        current: world,
        origin,
        dragged: false,
        mode: MarqueeMode::held(input.modifiers),
    });
}

fn is_group(app: &App, id: &EntityId) -> bool {
    app.document.entity(id).is_some_and(crate::scope::is_group)
}
