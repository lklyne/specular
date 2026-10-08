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
