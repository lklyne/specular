use super::*;

fn rect_at(x: f32, y: f32) -> ScreenRect {
    ScreenRect::new(x, y, 200.0, 100.0)
}

fn id(name: &str) -> EntityId {
    EntityId::from(name)
}

/// Entities a, b and c at the screen positions the Electron test uses.
fn world() -> Vec<(EntityId, ScreenRect)> {
    vec![
        (id("a"), rect_at(0.0, 0.0)),
        (id("b"), rect_at(400.0, 0.0)),
        (id("c"), rect_at(0.0, 300.0)),
    ]
}

fn lookup(world: &[(EntityId, ScreenRect)]) -> impl Fn(&EntityId) -> Option<ScreenRect> + '_ {
    move |wanted| {
        (world.iter())
            .find(|(entity, _)| entity == wanted)
            .map(|(_, rect)| *rect)
    }
}

fn bodies(world: &[(EntityId, ScreenRect)]) -> Vec<Body<'_>> {
    (world.iter())
        .map(|(id, rect)| Body { id, rect: *rect })
        .collect()
}

fn a_to_b() -> Edge {
    Edge {
        from_side: Some(EdgeSide::Right),
        to_side: Some(EdgeSide::Left),
        ..Edge::new("e1", "a", "b")
    }
}

fn start(entity: &str, side: EdgeSide, edges: &[Edge], cursor: Vec2) -> State {
    let world = world();
    begin(&id(entity), side, cursor, edges.iter(), lookup(&world))
}

fn dragged(mut state: State, cursor: Vec2) -> State {
    let world = world();
    update(&mut state, cursor, &bodies(&world), 1.0);
    state
}

#[test]
fn an_anchor_with_no_edge_starts_a_create() {
    let state = start("a", EdgeSide::Right, &[], Vec2::new(250.0, 50.0));
    assert_eq!(
        state,
        State::Create {
            from: id("a"),
            from_side: EdgeSide::Right,
            cursor: Vec2::new(250.0, 50.0),
            snap: None,
        }
    );
}

#[test]
fn an_anchor_an_edge_ends_on_starts_an_edit_of_that_end() {
    let state = start("b", EdgeSide::Left, &[a_to_b()], Vec2::new(410.0, 50.0));
    assert_eq!(
        state,
        State::Edit {
            edge: EdgeId::from("e1"),
            moving: End::To,
            fixed: id("a"),
            fixed_side: EdgeSide::Right,
            cursor: Vec2::new(410.0, 50.0),
            snap: None,
        }
    );
}

#[test]
fn an_edge_with_no_sides_is_found_by_the_sides_it_faces() {
    let bare = Edge::new("e1", "a", "b");
    let state = start("a", EdgeSide::Right, &[bare], Vec2::new(208.0, 50.0));
    assert!(matches!(
        state,
        State::Edit {
            moving: End::From,
            ..
        }
    ));
}

#[test]
fn the_pointer_snaps_to_an_anchor_within_reach() {
    let state = dragged(
        start("a", EdgeSide::Right, &[], Vec2::new(250.0, 50.0)),
        Vec2::new(392.0, 50.0),
    );
    assert_eq!(
        state.snap(),
        Some(&Snap {
            entity: id("b"),
            side: EdgeSide::Left
        })
    );
}

#[test]
fn the_snap_clears_when_the_pointer_leaves_its_reach() {
    let state = start("a", EdgeSide::Right, &[], Vec2::new(250.0, 50.0));
    let state = dragged(
        dragged(state, Vec2::new(392.0, 50.0)),
        Vec2::new(250.0, 250.0),
    );
    assert_eq!(state.snap(), None);
}

#[test]
fn the_entity_a_drag_hangs_off_is_never_a_target() {
    let state = dragged(
        start("a", EdgeSide::Right, &[], Vec2::new(208.0, 50.0)),
        Vec2::new(208.0, 50.0),
    );
    assert_eq!(state.snap(), None);
}

#[test]
fn reach_shrinks_with_the_zoom_down_to_a_third_of_it() {
    let world = world();
    let mut state = start("a", EdgeSide::Right, &[], Vec2::ZERO);
    // 40 px from b's left dot: inside 48, outside 48 * 0.5.
    let cursor = Vec2::new(352.0, 50.0);
    update(&mut state, cursor, &bodies(&world), 1.0);
    assert!(state.snap().is_some());
    update(&mut state, cursor, &bodies(&world), 0.5);
    assert!(state.snap().is_none());
    // Below the floor the reach stays at 48 * 0.35 = 16.8.
    update(&mut state, Vec2::new(370.0, 50.0), &bodies(&world), 0.1);
    assert!(state.snap().is_none());
    update(&mut state, Vec2::new(380.0, 50.0), &bodies(&world), 0.1);
    assert!(state.snap().is_some());
}

#[test]
fn releasing_a_create_on_an_anchor_makes_an_edge() {
    let state = dragged(
        start("a", EdgeSide::Right, &[], Vec2::new(250.0, 50.0)),
        Vec2::new(392.0, 50.0),
    );
    assert_eq!(
        commit(&state),
        Outcome::Create {
            from: id("a"),
            from_side: EdgeSide::Right,
            to: id("b"),
            to_side: EdgeSide::Left,
        }
    );
}

#[test]
fn releasing_a_create_on_nothing_does_nothing() {
    let state = start("a", EdgeSide::Right, &[], Vec2::new(250.0, 250.0));
    assert_eq!(commit(&state), Outcome::Noop);
}

#[test]
fn releasing_an_edit_on_an_anchor_moves_that_end() {
    let state = dragged(
        start("b", EdgeSide::Left, &[a_to_b()], Vec2::new(410.0, 50.0)),
        Vec2::new(100.0, 292.0),
    );
    assert_eq!(
        commit(&state),
        Outcome::Edit {
            edge: EdgeId::from("e1"),
            moving: End::To,
            target: id("c"),
            target_side: EdgeSide::Top,
        }
    );
}

#[test]
fn releasing_an_edit_on_nothing_deletes_the_edge() {
    let state = dragged(
        start("b", EdgeSide::Left, &[a_to_b()], Vec2::new(410.0, 50.0)),
        Vec2::new(1000.0, 1000.0),
    );
    assert_eq!(commit(&state), Outcome::Discard(EdgeId::from("e1")));
}

#[test]
fn cancelling_an_edit_deletes_the_edge_and_cancelling_a_create_does_nothing() {
    let edit = start("b", EdgeSide::Left, &[a_to_b()], Vec2::new(410.0, 50.0));
    let create = start("a", EdgeSide::Right, &[], Vec2::new(250.0, 50.0));
    assert_eq!(cancel(&edit), Outcome::Discard(EdgeId::from("e1")));
    assert_eq!(cancel(&create), Outcome::Noop);
}

#[test]
fn the_origin_is_the_grabbed_anchor_of_a_create_and_the_far_end_of_an_edit() {
    let create = start("a", EdgeSide::Right, &[], Vec2::ZERO);
    let to_end = start("b", EdgeSide::Left, &[a_to_b()], Vec2::ZERO);
    let from_end = start("a", EdgeSide::Right, &[a_to_b()], Vec2::ZERO);
    assert_eq!(create.origin(), (&id("a"), EdgeSide::Right));
    assert_eq!(to_end.origin(), (&id("a"), EdgeSide::Right));
    assert_eq!(from_end.origin(), (&id("b"), EdgeSide::Left));
}
