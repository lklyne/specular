//! Resizing from the handles: one entity from each of its eight, the
//! per-kind minimum and aspect rules, text and drawings, a selection of
//! several scaled together, and the cursor over a handle.

use specular_core::CssSize;
use specular_doc::{
    Color, Drawing, Entity, EntityId, FileRef, JsonMap, Kind, PageAnchor, Point, Rect, Stroke,
    Text, TextStyle, WidthMode,
};
use specular_interact::{Cursor, Effect, Key, PageNotice, shown_rect};
use specular_testkit::{
    SHIFT, TestApp, assert_doc_snapshot, drawing, file, group, inside, page, shape, sticky, text,
};

const S: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
/// The handles of [`S`]: corners clockwise from the top left, then the
/// middles of the sides clockwise from the top.
const TOP_LEFT: (f32, f32) = (100.0, 100.0);
const TOP_RIGHT: (f32, f32) = (300.0, 100.0);
const BOTTOM_RIGHT: (f32, f32) = (300.0, 200.0);
const BOTTOM_LEFT: (f32, f32) = (100.0, 200.0);
const TOP: (f32, f32) = (200.0, 100.0);
const RIGHT: (f32, f32) = (300.0, 150.0);
const BOTTOM: (f32, f32) = (200.0, 200.0);
const LEFT: (f32, f32) = (100.0, 150.0);

/// `entity` alone on the canvas, selected, so its handles exist.
fn selected(entity: Entity) -> TestApp {
    let id = entity.id.as_str().to_owned();
    let mut app = TestApp::with_entities([entity]);
    app.select(&[id.as_str()]);
    app
}

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

/// A text's size and width mode.
fn type_of(app: &TestApp, id: &str) -> (Option<f64>, Option<WidthMode>) {
    let Kind::Text(text) = &app.entity(id).kind else {
        return (None, None);
    };
    (text.size, text.width_mode)
}

fn viewport_effects(app: &mut TestApp) -> Vec<Effect> {
    let lays_out = |effect: &Effect| matches!(effect, Effect::SetPageViewport { .. });
    app.take_effects().into_iter().filter(lays_out).collect()
}

fn cursor_effects(app: &mut TestApp) -> Vec<Cursor> {
    (app.take_effects().into_iter())
        .filter_map(|effect| match effect {
            Effect::SetCursor(cursor) => Some(cursor),
            _ => None,
        })
        .collect()
}

#[test]
fn each_handle_moves_its_own_edges_and_holds_the_others() {
    // Every handle is dragged 40 right and 20 down.
    let cases = [
        (TOP_LEFT, Rect::new(140.0, 120.0, 160.0, 80.0)),
        (TOP_RIGHT, Rect::new(100.0, 120.0, 240.0, 80.0)),
        (BOTTOM_RIGHT, Rect::new(100.0, 100.0, 240.0, 120.0)),
        (BOTTOM_LEFT, Rect::new(140.0, 100.0, 160.0, 120.0)),
        (TOP, Rect::new(100.0, 120.0, 200.0, 80.0)),
        (RIGHT, Rect::new(100.0, 100.0, 240.0, 100.0)),
        (BOTTOM, Rect::new(100.0, 100.0, 200.0, 120.0)),
        (LEFT, Rect::new(140.0, 100.0, 160.0, 100.0)),
    ];
    for (handle, expected) in cases {
        let mut app = selected(shape("s", S));
        app.drag(handle, (handle.0 + 40.0, handle.1 + 20.0));
        assert_eq!(app.rect("s"), expected, "from the handle at {handle:?}");
        app.assert_undo_returns_to_start();
    }
}

#[test]
fn a_press_off_the_centre_of_a_handle_does_not_make_the_rect_jump() {
    let mut app = selected(shape("s", S));
    app.press((306.0, 206.0)).drag_to((311.0, 211.0)).release();
    assert_eq!((app.rect("s"), app.app().can_undo()), (S, false));
}

#[test]
fn every_kind_stops_at_its_own_minimum_size() {
    let square = Rect::new(100.0, 100.0, 400.0, 400.0);
    let cases = [
        (page("e", square), (320.0, 200.0)),
        // 8 px type, and a note that small is 115 tall.
        (text("e", square), (100.0, 115.0)),
        (file("e", square), (80.0, 80.0)),
        (group("e", square), (120.0, 80.0)),
        // 16 is the limit, and the grid line past it is the first place the
        // edge can stop.
        (drawing("e", square), (20.0, 20.0)),
        (shape("e", square), (24.0, 24.0)),
    ];
    for (entity, minimum) in cases {
        let kind = entity.kind.name();
        let mut app = selected(entity);
        app.drag((500.0, 500.0), (0.0, 0.0));
        let rect = app.rect("e");
        assert_eq!((rect.width, rect.height), minimum, "for a {kind}");
        assert_eq!((rect.x, rect.y), (100.0, 100.0), "for a {kind}");
        app.assert_undo_returns_to_start();
    }
}

#[test]
fn shift_flips_whether_a_resize_keeps_the_ratio() {
    let notes = || Entity {
        kind: Kind::File(FileRef {
            file: "notes.md".to_owned(),
            ..FileRef::default()
        }),
        ..file("f", S)
    };
    let free = Rect::new(100.0, 100.0, 300.0, 120.0);
    let locked = Rect::new(100.0, 100.0, 300.0, 150.0);
    // A shape and a document are free until Shift; a picture is locked until it.
    let cases = [
        ("shape", shape("f", S), false, free),
        ("shape", shape("f", S), true, locked),
        ("picture", file("f", S), false, locked),
        ("picture", file("f", S), true, free),
        ("document", notes(), false, free),
        ("document", notes(), true, locked),
    ];
    for (name, entity, shift, expected) in cases {
        let mut app = selected(entity);
        if shift {
            app.hold(SHIFT).drag(BOTTOM_RIGHT, (400.0, 220.0)).let_go();
        } else {
            app.drag(BOTTOM_RIGHT, (400.0, 220.0));
        }
        assert_eq!(app.rect("f"), expected, "{name}, shift {shift}");
        app.assert_undo_returns_to_start();
    }
}

// Text: the sides reflow, everything else scales the type, and the height
// is the content's either way. A sticky is at least 200 tall at 14 px type.

#[test]
fn a_text_side_handle_changes_the_width_and_keeps_the_type_size() {
    let mut app = selected(text("t", S));
    app.drag(RIGHT, (400.0, 150.0));
    assert_eq!(
        (app.rect("t"), type_of(&app, "t")),
        (
            Rect::new(100.0, 100.0, 300.0, 200.0),
            (None, Some(WidthMode::Fixed))
        )
    );
    app.assert_undo_returns_to_start();
    // The left handle reflows too: the right edge stays at 300.
    let mut app = selected(text("t", S));
    app.drag(LEFT, (0.0, 150.0));
    assert_eq!(
        (app.rect("t"), type_of(&app, "t")),
        (
            Rect::new(0.0, 100.0, 300.0, 200.0),
            (None, Some(WidthMode::Fixed))
        )
    );
}

#[test]
fn a_narrowed_text_grows_as_tall_as_its_wrapped_lines_in_one_undo_step() {
    // 24 characters at 10 units each: one 20-unit line until it has to wrap.
    let words = Kind::Text(Text {
        text: "aaaa bbbb cccc dddd eeee".to_owned(),
        style: Some(TextStyle::Plain),
        width_mode: Some(WidthMode::Auto),
        ..Text::default()
    });
    let mut app = selected(Entity::new(
        "t",
        Rect::new(100.0, 100.0, 300.0, 20.0),
        words,
    ));
    // The resize fixes the width mode as well as the rect, and both are the
    // one undo step. 120 wide wraps at 112: two words a line, three lines.
    app.press((400.0, 110.0)).drag_to((220.0, 110.0));
    assert_eq!(app.rect("t"), Rect::new(100.0, 100.0, 120.0, 60.0));
    app.release().undo();
    assert_eq!(
        (app.rect("t"), app.app().can_undo()),
        (Rect::new(100.0, 100.0, 300.0, 20.0), false)
    );
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn a_text_corner_scales_the_type_with_the_width() {
    let mut app = selected(text("t", S));
    app.drag(BOTTOM_RIGHT, (500.0, 300.0));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"t","type":"text","x":100,"y":100,"width":400,"height":400,"text":"t","specular":{"widthMode":"fixed","textSize":28}}
    edges:
    specular: {"entityOrder":["t"]}
    "#);
    app.assert_undo_returns_to_start();
}

#[test]
fn the_type_size_stays_within_its_limits() {
    let sized = |size: f64| Entity {
        kind: Kind::Text(Text {
            text: "t".to_owned(),
            size: Some(size),
            ..Text::default()
        }),
        ..text("t", S)
    };
    let mut grown = selected(sized(200.0));
    grown.drag(BOTTOM_RIGHT, (500.0, 300.0));
    let mut shrunk = selected(sized(10.0));
    shrunk.drag(BOTTOM_RIGHT, (100.0, 100.0));
    assert_eq!(
        (type_of(&grown, "t").0, type_of(&shrunk, "t").0),
        (Some(256.0), Some(8.0))
    );
    grown.assert_undo_returns_to_start();
}

// Drawings and groups.

#[test]
fn a_drawing_stretches_its_points_into_the_new_box() {
    let box_ = Rect::new(100.0, 100.0, 100.0, 100.0);
    let mut app = selected(ink(
        "d",
        box_,
        &[(100.0, 100.0), (150.0, 100.0), (200.0, 200.0)],
    ));
    // The top-left corner moves, so the points translate as well as scale.
    app.drag((100.0, 100.0), (60.0, 40.0));
    assert_eq!(
        (app.rect("d"), points(&app, "d")),
        (
            Rect::new(60.0, 40.0, 140.0, 160.0),
            vec![(60.0, 40.0), (130.0, 40.0), (200.0, 200.0)]
        )
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn resizing_a_group_alone_moves_its_border_and_not_its_members() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(100.0, 100.0, 400.0, 300.0)),
        inside("g", shape("a", Rect::new(200.0, 200.0, 100.0, 100.0))),
    ]);
    app.select(&["g"]).drag((100.0, 100.0), (60.0, 40.0));
    assert_eq!(
        (app.rect("g"), app.rect("a")),
        (
            Rect::new(60.0, 40.0, 440.0, 360.0),
            Rect::new(200.0, 200.0, 100.0, 100.0)
        )
    );
    app.assert_undo_returns_to_start();
}

// A page is laid out at its rect's size as the handle moves.

#[test]
fn a_page_is_laid_out_at_each_size_its_side_handle_is_dragged_to() {
    let mut app = selected(page("p", Rect::new(100.0, 100.0, 400.0, 300.0)));
    let lays_out = |width| Effect::SetPageViewport {
        page: EntityId::from("p"),
        viewport: CssSize::new(width, 300),
    };
    app.press((500.0, 250.0))
        .drag_to((600.0, 250.0))
        .drag_to((700.0, 250.0));
    let during = viewport_effects(&mut app);
    let live = app.app().page_placement(&EntityId::from("p")).unwrap();
    let after = viewport_effects(app.release());
    assert_eq!(
        (during, live.viewport, live.rect.width, after),
        (
            vec![lays_out(500), lays_out(600)],
            CssSize::new(600, 300),
            600.0,
            Vec::new()
        )
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_page_resize_abandoned_lays_the_page_out_as_it_started() {
    let mut app = selected(page("p", Rect::new(100.0, 100.0, 400.0, 300.0)));
    app.press((500.0, 250.0)).drag_to((700.0, 250.0));
    viewport_effects(&mut app);
    let after = viewport_effects(app.key(Key::Escape));
    assert_eq!(
        (after, app.rect("p")),
        (
            vec![Effect::SetPageViewport {
                page: EntityId::from("p"),
                viewport: CssSize::new(400, 300)
            }],
            Rect::new(100.0, 100.0, 400.0, 300.0)
        )
    );
}

// A selection of several scales inside its bounds.

/// Two shapes side by side, both selected. Their bounds are 300 by 100 with
/// the bottom-right corner at (400, 200).
fn pair() -> TestApp {
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(300.0, 100.0, 100.0, 100.0)),
    ]);
    app.select(&["a", "b"]);
    app
}

#[test]
fn a_selection_of_several_scales_every_entity_inside_its_bounds() {
    // Bounds (100, 100) to (400, 300), the shapes offset on both axes.
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(300.0, 200.0, 100.0, 100.0)),
    ]);
    app.select(&["a", "b"]);
    // Twice as wide and three times as tall.
    app.drag((400.0, 300.0), (700.0, 700.0));
    assert_eq!(
        (app.rect("a"), app.rect("b")),
        (
            Rect::new(100.0, 100.0, 200.0, 300.0),
            Rect::new(500.0, 400.0, 200.0, 300.0)
        )
    );
    assert_eq!(app.selected_ids(), ["a", "b"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_side_handle_of_the_bounds_scales_one_axis() {
    let mut app = pair();
    // The left side of the bounds, dragged 300 further left.
    app.drag((100.0, 150.0), (-200.0, 150.0));
    assert_eq!(
        (app.rect("a"), app.rect("b")),
        (
            Rect::new(-200.0, 100.0, 200.0, 100.0),
            Rect::new(200.0, 100.0, 200.0, 100.0)
        )
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn the_bounds_stop_at_twenty_units_and_nothing_inside_vanishes() {
    // A speck between the two shapes rounds to nothing at this scale.
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(300.0, 100.0, 100.0, 100.0)),
        shape("c", Rect::new(220.0, 100.0, 5.0, 5.0)),
    ]);
    app.select(&["a", "b", "c"]);
    app.drag((400.0, 200.0), (-900.0, -900.0));
    assert_eq!(
        (app.rect("a"), app.rect("b"), app.rect("c")),
        (
            Rect::new(100.0, 100.0, 7.0, 20.0),
            Rect::new(113.0, 100.0, 7.0, 20.0),
            Rect::new(108.0, 100.0, 1.0, 1.0)
        )
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_group_in_the_selection_scales_with_everything_inside_it() {
    let box_ = Rect::new(120.0, 220.0, 100.0, 50.0);
    let mut app = TestApp::with_entities([
        group("g", Rect::new(100.0, 100.0, 300.0, 200.0)),
        inside("g", shape("a", Rect::new(120.0, 120.0, 100.0, 80.0))),
        inside("g", ink("d", box_, &[(120.0, 220.0), (220.0, 270.0)])),
        shape("x", Rect::new(500.0, 100.0, 100.0, 100.0)),
    ]);
    // Bounds (100, 100) to (600, 300); the corner is dragged to double them.
    app.select(&["g", "x"])
        .drag((600.0, 300.0), (1100.0, 500.0));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"g","type":"group","x":100,"y":100,"width":600,"height":400}
      {"id":"a","type":"shape","x":140,"y":140,"width":200,"height":160,"shapeKind":"rectangle","text":"","parentGroupId":"g"}
      {"id":"d","type":"drawing","x":140,"y":340,"width":200,"height":100,"strokes":[{"id":"d-stroke","color":"neutral","width":2,"points":[{"x":140,"y":340},{"x":340,"y":440}]}],"parentGroupId":"g"}
      {"id":"x","type":"shape","x":900,"y":100,"width":200,"height":200,"shapeKind":"rectangle","text":""}
    edges:
    specular: {"entityOrder":["g","a","d","x"]}
    "#);
    app.assert_undo_returns_to_start();
}

#[test]
fn pages_in_a_scaled_selection_are_laid_out_as_it_scales() {
    let mut app = TestApp::with_entities([
        page("p", Rect::new(100.0, 100.0, 400.0, 300.0)),
        shape("s", Rect::new(600.0, 100.0, 100.0, 100.0)),
    ]);
    app.select(&["p", "s"])
        .press((700.0, 400.0))
        .drag_to((1300.0, 700.0));
    let during = viewport_effects(&mut app);
    let after = viewport_effects(app.release());
    assert_eq!(
        (during, after, app.rect("p")),
        (
            vec![Effect::SetPageViewport {
                page: EntityId::from("p"),
                viewport: CssSize::new(800, 600)
            }],
            Vec::new(),
            Rect::new(100.0, 100.0, 800.0, 600.0)
        )
    );
    app.assert_undo_returns_to_start();
}

// The cursor says which way a handle resizes.

#[test]
fn the_cursor_follows_the_handle_under_the_pointer() {
    let mut app = selected(shape("s", S));
    app.pointer_move(BOTTOM_RIGHT)
        .pointer_move((302.0, 202.0))
        .pointer_move(TOP_RIGHT)
        .pointer_move(TOP_LEFT)
        .pointer_move(BOTTOM_LEFT)
        .pointer_move(RIGHT)
        .pointer_move((200.0, 150.0));
    assert_eq!(
        cursor_effects(&mut app),
        [
            Cursor::ResizeNwse,
            Cursor::ResizeNesw,
            Cursor::ResizeNwse,
            Cursor::ResizeNesw,
            Cursor::ResizeEw,
            Cursor::Default
        ]
    );
}

// Resizing what its page has carried (the scroll rebase).

/// `entity` hooked to the page `p1` (400x300 at (100, 100)) at no scroll,
/// with the page then scrolled 40 down: it is seen 40 above where it is
/// stored.
fn carried(entities: Vec<Entity>, free: Vec<Entity>) -> TestApp {
    let hooked = entities.into_iter().map(|entity| Entity {
        anchor: Some(PageAnchor {
            page_url: Some("https://example.com/p1".to_owned()),
            scroll_x: Some(0.0),
            scroll_y: Some(0.0),
            ..PageAnchor::new("p1".into())
        }),
        ..entity
    });
    let mut all = vec![page("p1", Rect::new(100.0, 100.0, 400.0, 300.0))];
    all.extend(hooked);
    all.extend(free);
    let mut app = TestApp::with_entities(all);
    app.page_reports("p1", PageNotice::Scrolled { x: 0.0, y: 40.0 });
    app
}

#[test]
fn a_resize_works_on_what_is_seen_and_stores_it_there() {
    // Each is stored at (200, 200) 100x100, so seen at (200, 160), and its
    // bottom-right handle at (300, 260) is dragged 40 right and 40 down.
    let stored = Rect::new(200.0, 200.0, 100.0, 100.0);
    let rows = [
        shape("s", stored),
        ink("s", stored, &[(200.0, 200.0), (300.0, 300.0)]),
        sticky("s", stored, "hello"),
    ];
    for entity in rows {
        let kind = entity.kind.clone();
        let mut app = carried(vec![entity], Vec::new());
        app.select(&["s"]).drag((300.0, 260.0), (340.0, 300.0));
        // A text's height is its content's, so only its corner and width
        // are the drag's.
        let rect = app.rect("s");
        assert_eq!(
            (rect.x, rect.y, rect.width),
            (200.0, 160.0, 140.0),
            "{kind:?}"
        );
        if !matches!(kind, Kind::Text(_)) {
            assert_eq!(rect.height, 140.0, "{kind:?}");
        }
        let anchor = app.entity("s").anchor.clone().expect("still hooked");
        assert_eq!(
            (anchor.page_id.as_str(), anchor.scroll_y),
            ("p1", Some(40.0)),
            "restamped at the scroll it was folded at"
        );
        assert_eq!(
            shown_rect(app.app(), app.entity("s")),
            Some(app.rect("s")),
            "it does not jump when the button comes up"
        );
        if matches!(kind, Kind::Drawing(_)) {
            assert_eq!(points(&app, "s"), [(200.0, 160.0), (340.0, 300.0)]);
        }
        app.undo();
        assert!(
            !app.app().can_undo(),
            "the resize and the fold are one step"
        );
        app.redo().assert_undo_returns_to_start();
    }
}

#[test]
fn a_selection_resized_while_one_of_it_is_carried_scales_what_is_seen() {
    // `a` is seen at (200, 160); `b` is free at (320, 160). Together they
    // are seen 220x100, and the corner at (420, 260) is dragged to twice
    // that.
    let mut app = carried(
        vec![shape("a", Rect::new(200.0, 200.0, 100.0, 100.0))],
        vec![shape("b", Rect::new(320.0, 160.0, 100.0, 100.0))],
    );
    app.select(&["a", "b"]).drag((420.0, 260.0), (640.0, 360.0));
    assert_eq!(app.rect("a"), Rect::new(200.0, 160.0, 200.0, 200.0));
    assert_eq!(app.rect("b"), Rect::new(440.0, 160.0, 200.0, 200.0));
    app.assert_undo_returns_to_start();
}
