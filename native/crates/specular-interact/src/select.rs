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
use specular_doc::{EdgeId, EntityId, ItemId};

use crate::comment;
use crate::marquee::MarqueeMode;
use crate::move_drag::{self, Click};
use crate::{
    App, Effect, Gesture, Hit, PointerInput, TextEdit, edge_drag, edit, groups, hit, resize_drag,
};

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
    match hit::hit_test(app, input.screen) {
        Hit::PageContent { page, .. } if app.session.focus.page() == Some(&page) => return false,
        Hit::PageContent { page, .. } => press_page(app, page, world, input, click_count),
        Hit::Handle { owner, handle } => {
            app.session.gesture =
                resize_drag::begin(app, owner, handle, world).map(Gesture::Resize);
        }
        // A double click on a group's title renames it.
        Hit::GroupLabel { group } if click_count > 1 && !is_additive(input.modifiers) => {
            edit::begin(app, &group, false, effects);
        }
        Hit::GroupLabel { group } | Hit::GroupBorder { group } => {
            begin_move(app, &group, world, input, None);
        }
        // A double click on a text, a sticky, a shape or a Document edits
        // its text.
        Hit::EntityBody { entity }
            if click_count > 1 && !is_additive(input.modifiers) && has_text(app, &entity) =>
        {
            edit::begin(app, &entity, false, effects);
            // A Document opens with the caret where it was clicked.
            if app.text_edit().is_some_and(TextEdit::is_note)
                && let Some(drag) = edit::press(app, input, 1)
            {
                app.session.gesture = Some(drag.into());
            }
        }
        Hit::EntityBody { entity } => press_body(app, entity, world, input, click_count),
        // A double click on an edge edits its label.
        Hit::Edge { edge } if click_count > 1 && !is_additive(input.modifiers) => {
            edit::begin_edge_label(app, &edge, effects);
        }
        Hit::Edge { edge } => press_edge(app, edge, world, input),
        Hit::Empty => begin_marquee(app, None, world, input),
        // The press was offered to the marks first; a mark here has gone
        // since.
        Hit::Comment { annotation } => comment::focus(app, Some(&annotation), effects),
        // The panels take their presses before a tool is offered one.
        Hit::Panel { .. } => {}
        // A drag from an anchor draws an edge, or moves the end of one.
        Hit::Anchor { entity, side } => {
            app.session.gesture = Some(edge_drag::begin(app, &entity, side, input.screen).into());
        }
    }
    true
}

/// A press on an edge's line. Where the line crosses an entity the two
/// share the press: a drag moves the entity, and a click selects the edge.
/// The edge alone would make the part of an entity under it dead to a drag.
fn press_edge(app: &mut App, edge: EdgeId, world: DVec2, input: &PointerInput) {
    let item = ItemId::Edge(edge.clone());
    if is_additive(input.modifiers) {
        app.session.selection.toggle(item);
    } else if let Some(under) = hit::entity_under_edges(app, input.screen) {
        begin_move(app, &under, world, input, Some(Click::SelectEdge(edge)));
    } else {
        app.session.selection.set([item]);
    }
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
        let click = enters.then(|| Click::Enter(page.clone()));
        begin_move(app, &page, world, input, click);
    }
}

fn press_body(
    app: &mut App,
    entity: EntityId,
    world: DVec2,
    input: &PointerInput,
    click_count: u8,
) {
    let modifiers = input.modifiers;
    let item = ItemId::Entity(entity.clone());
    // A double click on a group steps into it: its members, one level down,
    // become the selection.
    let enters = click_count > 1 && !is_additive(modifiers) && is_group(app, &entity);
    if enters && groups::enter(app, &entity) {
        return;
    }
    if modifiers.meta || modifiers.control {
        begin_marquee(app, Some(entity), world, input);
    } else if modifiers.shift {
        app.session.selection.toggle(item);
    } else if is_group(app, &entity) && !app.session.selection.contains(&item) {
        // A group's interior is canvas until the group is selected: a drag
        // marquees what is inside, and a click selects the group.
        begin_marquee(app, Some(entity), world, input);
    } else {
        begin_move(app, &entity, world, input, None);
    }
}

/// Selects `pressed` alone, unless the press is on the selection already (a
/// member, or anything that moves with one), which it then keeps. Either way
/// a drag from here moves the selection.
///
/// A press on one of several selected entities keeps them all for the drag.
/// If it turns out to be a click, it narrows the selection to that one.
/// `click` is what the click does instead, when it has something else to do.
fn begin_move(
    app: &mut App,
    pressed: &EntityId,
    world: DVec2,
    input: &PointerInput,
    click: Option<Click>,
) {
    let item = ItemId::Entity(pressed.clone());
    let held = app.selection_scope().holds(pressed);
    let alone = app.session.selection.items() == [item.clone()];
    if !held {
        app.session.selection.set([item]);
    }
    let click = click.unwrap_or_else(|| {
        if held && !alone {
            Click::SelectAlone(pressed.clone())
        } else {
            Click::Keep
        }
    });
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

fn has_text(app: &App, id: &EntityId) -> bool {
    app.text_frame(id).is_some()
}

fn is_group(app: &App, id: &EntityId) -> bool {
    app.document.entity(id).is_some_and(crate::scope::is_group)
}
