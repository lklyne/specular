//! Moving the selection by dragging a body: any kind, groups and what is
//! hooked to a page, the grid, Shift's axis lock, and Option-drag copies.

use specular_core::CssSize;
use specular_doc::{
    Color, Drawing, Entity, EntityId, JsonMap, Kind, PageAnchor, Point, Rect, Stroke,
};
use specular_interact::Effect;
use specular_testkit::{
    ALT, SHIFT, TestApp, assert_doc_snapshot, connected, document, drawing, group, inside, page,
    shape, text,
};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
const B: Rect = Rect::new(400.0, 100.0, 200.0, 100.0);
const C: Rect = Rect::new(100.0, 400.0, 100.0, 100.0);
/// A point on the body of `a`.
const ON_A: (f32, f32) = (150.0, 150.0);

/// Two texts side by side and a shape below them.
fn notes() -> TestApp {
    TestApp::with_entities([text("a", A), text("b", B), shape("c", C)])
}

/// A drawing `id` with one stroke through `points`, boxed at `rect`.
fn ink(id: &str, rect: Rect, points: &[(f64, f64)]) -> Entity {
    let stroke = Stroke {
        id: format!("{id}-stroke"),
        color: Color::Neutral,
        width: 2.0,
        points: points.iter().map(|(x, y)| Point::new(*x, *y)).collect(),
        brush: None,
        extra: JsonMap::new(),
    };
    Entity {
        kind: Kind::Drawing(Drawing {
            strokes: vec![stroke],
        }),
        ..drawing(id, rect)
    }
}

fn points(app: &TestApp, id: &str) -> Vec<(f64, f64)> {
    let Kind::Drawing(drawing) = &app.entity(id).kind else {
        return Vec::new();
    };
    (drawing.strokes.iter())
        .flat_map(|stroke| stroke.points.iter().map(|point| (point.x, point.y)))
        .collect()
}

/// The page effects, without the saves and cursor changes around them.
fn page_effects(app: &mut TestApp) -> Vec<Effect> {
    let hosts = |effect: &Effect| {
        matches!(
            effect,
            Effect::CreatePage { .. } | Effect::ClosePage(_) | Effect::SetPageViewport { .. }
        )
    };
    app.take_effects().into_iter().filter(hosts).collect()
}

#[test]
fn a_drag_on_an_unselected_body_selects_it_and_moves_it() {
    let mut app = notes();
    app.drag(ON_A, (250.0, 190.0));
    assert_eq!(
        (app.selected(), app.rect("a"), app.rect("b")),
        (Some("a"), Rect::new(200.0, 140.0, 200.0, 100.0), B)
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_drag_from_one_selected_entity_moves_the_whole_selection() {
    let mut app = notes();
    app.select(&["a", "c"]).drag(ON_A, (250.0, 190.0));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"text","x":200,"y":140,"width":200,"height":100,"text":"a"}
      {"id":"b","type":"text","x":400,"y":100,"width":200,"height":100,"text":"b"}
      {"id":"c","type":"shape","x":200,"y":440,"width":100,"height":100,"shapeKind":"rectangle","text":""}
    edges:
    specular: {"entityOrder":["a","b","c"]}
    "#);
    assert_eq!(app.selected_ids(), ["a", "c"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn the_pressed_entity_lands_on_the_grid_and_the_rest_keep_their_offsets() {
    let off_grid = Rect::new(105.0, 100.0, 200.0, 100.0);
    let mut app = TestApp::with_entities([text("a", off_grid), text("b", B)]);
    // 33 right and 8 up: the corner at 105 goes to 140, and 92 rounds to 100.
    app.select(&["a", "b"]).drag(ON_A, (183.0, 142.0));
    assert_eq!(
        (app.rect("a"), app.rect("b")),
        (
            Rect::new(140.0, 100.0, 200.0, 100.0),
            Rect::new(435.0, 100.0, 200.0, 100.0)
        )
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn shift_keeps_the_move_on_the_axis_the_pointer_took_further() {
    let mut app = notes();
    // Shift goes down after the press: a Shift-press toggles instead.
    app.select(&["a"]).press(ON_A).hold(SHIFT);
    let across = app.drag_to((250.0, 190.0)).rect("a");
    let down = app.drag_to((190.0, 250.0)).rect("a");
    app.release().let_go();
    assert_eq!(
        (across, down),
        (
            Rect::new(200.0, 100.0, 200.0, 100.0),
            Rect::new(100.0, 200.0, 200.0, 100.0)
        )
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_group_takes_everything_inside_it() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(80.0, 80.0, 540.0, 440.0)),
        inside("g", text("a", A)),
        inside("g", group("inner", Rect::new(380.0, 80.0, 240.0, 140.0))),
        inside("inner", text("b", B)),
        shape("out", Rect::new(700.0, 400.0, 100.0, 100.0)),
    ]);
    // A point inside the selected group with no member under it.
    app.select(&["g"]).drag((300.0, 300.0), (340.0, 400.0));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"g","type":"group","x":120,"y":180,"width":540,"height":440}
      {"id":"a","type":"text","x":140,"y":200,"width":200,"height":100,"text":"a","specular":{"parentGroupId":"g"}}
      {"id":"inner","type":"group","x":420,"y":180,"width":240,"height":140,"parentGroupId":"g"}
      {"id":"b","type":"text","x":440,"y":200,"width":200,"height":100,"text":"b","specular":{"parentGroupId":"inner"}}
      {"id":"out","type":"shape","x":700,"y":400,"width":100,"height":100,"shapeKind":"rectangle","text":""}
    edges:
    specular: {"entityOrder":["g","a","inner","b","out"]}
    "#);
    assert_eq!(app.selected(), Some("g"));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_drag_on_a_group_title_or_border_moves_it() {
    let labelled = Entity {
        label: Some("Flow".to_owned()),
        ..group("g", Rect::new(80.0, 80.0, 540.0, 140.0))
    };
    let mut app = TestApp::with_entities([labelled, inside("g", text("a", A))]);
    app.drag((90.0, 70.0), (130.0, 70.0));
    // The band just inside the border, past the handle strip on the outline.
    app.drag((127.0, 150.0), (167.0, 150.0));
    assert_eq!(
        (app.selected(), app.rect("g").x, app.rect("a").x),
        (Some("g"), 160.0, 180.0)
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn what_is_hooked_to_a_page_moves_with_it() {
    let note = Entity {
        anchor: Some(PageAnchor::new(EntityId::from("p1"))),
        ..text("note", Rect::new(450.0, 50.0, 100.0, 100.0))
    };
    let mut app = TestApp::with_entities([page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)), note]);
    app.drag((200.0, 300.0), (300.0, 360.0));
    assert_eq!(
        (page_effects(&mut app), app.rect("p1"), app.rect("note")),
        (
            Vec::new(),
            Rect::new(200.0, 160.0, 400.0, 300.0),
            Rect::new(550.0, 110.0, 100.0, 100.0)
        )
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_drawing_takes_its_points_with_it_and_ignores_the_grid() {
    let box_ = Rect::new(100.0, 100.0, 60.0, 40.0);
    let mut app = TestApp::with_entities([ink("d", box_, &[(100.0, 100.0), (160.0, 140.0)])]);
    app.drag((130.0, 120.0), (163.0, 112.0));
    assert_eq!(
        (app.rect("d"), points(&app, "d")),
        (
            Rect::new(133.0, 92.0, 60.0, 40.0),
            vec![(133.0, 92.0), (193.0, 132.0)]
        )
    );
    app.assert_undo_returns_to_start();
}

// Option-drag: the originals stay and copies land where the drag ends.

#[test]
fn option_drag_leaves_a_copy_and_selects_it() {
    let mut app = notes();
    app.hold(ALT)
        .select(&["a"])
        .drag(ON_A, (250.0, 190.0))
        .let_go();
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"text","x":100,"y":100,"width":200,"height":100,"text":"a"}
      {"id":"b","type":"text","x":400,"y":100,"width":200,"height":100,"text":"b"}
      {"id":"c","type":"shape","x":100,"y":400,"width":100,"height":100,"shapeKind":"rectangle","text":""}
      {"id":"e220a8397b1dcdaf","type":"text","x":200,"y":140,"width":200,"height":100,"text":"a"}
    edges:
    specular: {"entityOrder":["a","b","c","e220a8397b1dcdaf"]}
    "#);
    let copy = app.selected().map(str::to_owned);
    assert!(copy.is_some_and(|id| id != "a"));
    app.assert_undo_returns_to_start();
}

#[test]
fn option_drag_on_a_group_copies_its_members_into_the_copy_and_the_edges_between_them() {
    let start = connected(
        connected(
            document([
                group("g", Rect::new(80.0, 80.0, 540.0, 140.0)),
                inside("g", text("a", A)),
                inside("g", text("b", B)),
                shape("c", C),
            ]),
            "inner",
            "a",
            "b",
        ),
        "outer",
        "a",
        "c",
    );
    let mut app = TestApp::from_document(start);
    app.hold(ALT)
        .select(&["g"])
        .drag(ON_A, (150.0, 450.0))
        .let_go();
    // `outer` leaves the group, so it is not copied.
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"g","type":"group","x":80,"y":80,"width":540,"height":140}
      {"id":"a","type":"text","x":100,"y":100,"width":200,"height":100,"text":"a","specular":{"parentGroupId":"g"}}
      {"id":"b","type":"text","x":400,"y":100,"width":200,"height":100,"text":"b","specular":{"parentGroupId":"g"}}
      {"id":"c","type":"shape","x":100,"y":400,"width":100,"height":100,"shapeKind":"rectangle","text":""}
      {"id":"e220a8397b1dcdaf","type":"group","x":80,"y":380,"width":540,"height":140}
      {"id":"6e789e6aa1b965f4","type":"text","x":100,"y":400,"width":200,"height":100,"text":"a","specular":{"parentGroupId":"e220a8397b1dcdaf"}}
      {"id":"06c45d188009454f","type":"text","x":400,"y":400,"width":200,"height":100,"text":"b","specular":{"parentGroupId":"e220a8397b1dcdaf"}}
    edges:
      {"id":"inner","fromNode":"a","toNode":"b"}
      {"id":"outer","fromNode":"a","toNode":"c"}
      {"id":"f88bb8a8724c81ec","fromNode":"6e789e6aa1b965f4","toNode":"06c45d188009454f"}
    specular: {"entityOrder":["g","a","b","c","inner","outer","e220a8397b1dcdaf","6e789e6aa1b965f4","06c45d188009454f","f88bb8a8724c81ec"]}
    "#);
    app.assert_undo_returns_to_start();
}

#[test]
fn option_drag_on_a_page_hosts_the_copy_and_undo_closes_it() {
    let mut app = TestApp::with_pages(1);
    app.hold(ALT).drag((200.0, 200.0), (200.0, 600.0)).let_go();
    let copy = EntityId::from(app.selected().unwrap());
    let created = page_effects(&mut app);
    let undone = page_effects(app.undo());
    assert_eq!(
        (created, undone),
        (
            vec![Effect::CreatePage {
                page: copy.clone(),
                url: "https://example.com/p1".to_owned(),
                viewport: CssSize::new(400, 300)
            }],
            vec![Effect::ClosePage(copy)]
        )
    );
    app.redo().assert_undo_returns_to_start();
}
