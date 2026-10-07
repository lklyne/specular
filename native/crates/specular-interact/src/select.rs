//! What a left press does with the select tool.
//!
//! Pages are select-first, interact-second (ADR 0022). The first press on a
//! page selects it. A press on the page that is already the whole selection
//! *enters* it once it is released without having become a drag: the page
//! takes keyboard focus, and from then on presses on its body go to the
//! page, modifiers and all. The entering click itself is not forwarded.
//! Escape, or selecting anything else, leaves the page.
//!
//! A press on a body also starts a move of the selection, which stays a
//! click until the pointer travels.

use glam::DVec2;
use specular_core::Modifiers;
use specular_doc::{EntityId, ItemId};

use crate::marquee::MarqueeMode;
use crate::move_drag::{self, Click};
use crate::{App, Gesture, Hit, PointerInput, hit, resize_drag};

/// Whether a click with these modifiers changes the selection item by item
/// instead of replacing it.
pub(crate) const fn is_additive(modifiers: Modifiers) -> bool {
    modifiers.shift || modifiers.meta || modifiers.control
}

/// Offers a left press to the select tool. Returns `false` only when the
/// press is on the body of the entered page, which gets it instead.
pub(crate) fn press(app: &mut App, input: &PointerInput, click_count: u8) -> bool {
    let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
    let hit = match hit::hit_test(app, input.screen) {
        // Edges cannot be drawn yet, so an anchor passes the press to what
        // is under it.
        Hit::Anchor { .. } => hit::body_at(app, input.screen),
        other => other,
    };
    match hit {
        Hit::PageContent { page, .. } if app.session.focus.page() == Some(&page) => return false,
        Hit::PageContent { page, .. } => press_page(app, page, world, input, click_count),
        Hit::Handle { owner, handle } => {
            app.session.gesture =
                resize_drag::begin(app, owner, handle, world).map(Gesture::Resize);
        }
        Hit::GroupLabel { group } | Hit::GroupBorder { group } => {
            begin_move(app, &group, world, input, false);
        }
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

fn press_page(app: &mut App, page: EntityId, world: DVec2, input: &PointerInput, click_count: u8) {
    let modifiers = input.modifiers;
    if modifiers.meta || modifiers.control {
        begin_marquee(app, Some(page), world, input);
    } else if modifiers.shift {
        app.session.selection.toggle(ItemId::Entity(page));
    } else {
        // The second click of a double-click enters however fast the two
        // landed, and whatever the first one found selected.
        let enters = click_count > 1 || app.session.selection.single_entity() == Some(&page);
        begin_move(app, &page, world, input, enters);
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
        begin_move(app, &entity, world, input, false);
    }
}

/// Selects `pressed` alone, unless the press is on the selection already (a
/// member, or anything that moves with one), which it then keeps. Either way
/// a drag from here moves the selection.
///
/// A press on one of several selected entities keeps them all for the drag.
/// If it turns out to be a click, it narrows the selection to that one.
/// `enters` makes the click enter the page pressed.
fn begin_move(app: &mut App, pressed: &EntityId, world: DVec2, input: &PointerInput, enters: bool) {
    let item = ItemId::Entity(pressed.clone());
    let held = app.selection_scope().holds(pressed);
    let alone = app.session.selection.items() == [item.clone()];
    if !held {
        app.session.selection.set([item]);
    }
    let click = if enters {
        Click::Enter(pressed.clone())
    } else if held && !alone {
        Click::SelectAlone(pressed.clone())
    } else {
        Click::Keep
    };
    app.session.gesture =
        move_drag::begin(app, pressed, world, input.screen, click).map(Gesture::Move);
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
