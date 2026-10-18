//! Delete, duplicate and the arrow-key nudge: each acts on the selection as
//! ADR 0034 resolves it, and each is one undo step.

use specular_core::{CssSize, InputEvent};
use specular_doc::{
    Annotation, AnnotationAnchor, Color, Drawing, Entity, EntityId, JsonMap, Kind, PageAnchor,
    Point, Rect, RegionAnchor, Stroke,
};
use specular_interact::{Action, Effect, Key, PageNotice};
use specular_testkit::{
    CMD, SHIFT, TestApp, assert_doc_snapshot, comment, connected, document, drawing, group, inside,
    page, shape, text, with_comment,
};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
const B: Rect = Rect::new(400.0, 100.0, 200.0, 100.0);
const C: Rect = Rect::new(100.0, 400.0, 100.0, 100.0);

/// Texts `a` and `b` and a shape `c`, with an edge from `a` to each.
fn linked() -> TestApp {
    let start = document([text("a", A), text("b", B), shape("c", C)]);
    TestApp::from_document(connected(connected(start, "ab", "a", "b"), "ac", "a", "c"))
}

fn ids(app: &TestApp) -> Vec<&str> {
    app.document()
        .order()
        .iter()
        .map(specular_doc::ItemId::as_str)
        .collect()
}

fn page_effects(app: &mut TestApp) -> Vec<Effect> {
    let hosts = |effect: &Effect| {
        matches!(
            effect,
            Effect::CreatePage { .. } | Effect::ClosePage(_) | Effect::SetPageViewport { .. }
        )
    };
    app.take_effects().into_iter().filter(hosts).collect()
}

// Delete.

#[test]
fn backspace_removes_the_selected_entity_and_every_edge_that_touched_it() {
    let mut app = linked();
    app.select(&["a"]).key(Key::Backspace);
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"b","type":"text","x":400,"y":100,"width":200,"height":100,"text":"b"}
      {"id":"c","type":"shape","x":100,"y":400,"width":100,"height":100,"shapeKind":"rectangle","text":""}
    edges:
    specular: {"entityOrder":["b","c"]}
    "#);
    assert_eq!(app.selected_ids(), [] as [&str; 0]);
    app.undo();
    assert!(!app.app().can_undo(), "the delete was one step");
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn an_edge_selected_along_with_its_end_is_removed_once() {
    let mut app = linked();
    app.select(&["ab", "b"]).key(Key::Backspace);
    assert_eq!(ids(&app), ["a", "c", "ac"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn deleting_a_page_closes_it_and_frees_what_was_hooked_to_it_where_it_is_seen() {
    let hooked = |entity: Entity| Entity {
        anchor: Some(PageAnchor {
            scroll_x: Some(0.0),
            scroll_y: Some(0.0),
            ..PageAnchor::new(EntityId::from("p1"))
        }),
        ..entity
    };
    let on_page = |annotation: Annotation| Annotation {
        page_anchor: Some(PageAnchor::new(EntityId::from("p1"))),
        ..annotation
    };
    let region = AnnotationAnchor::Region(RegionAnchor::Document {
        doc_rect: Rect::new(10.0, 10.0, 50.0, 50.0),
    });
    let start = document([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        hooked(text("note", Rect::new(200.0, 200.0, 100.0, 100.0))),
        hooked(shape("box", Rect::new(320.0, 200.0, 100.0, 100.0))),
        shape("c", Rect::new(100.0, 600.0, 100.0, 100.0)),
    ]);
    let start = with_comment(start, on_page(comment("region", region, "tighten")));
    let mut app = TestApp::from_document(start);
    // The page has carried both 40 up since they were placed.
    app.page_reports("p1", PageNotice::Scrolled { x: 0.0, y: 40.0 });
    app.select(&["p1"]).key(Key::Backspace);
    let closed = page_effects(&mut app);
    assert_eq!(closed, [Effect::ClosePage(EntityId::from("p1"))]);
    assert_eq!(ids(&app), ["note", "box", "c"], "only the page is removed");
    for (id, seen) in [("note", 200.0), ("box", 320.0)] {
        assert_eq!(app.entity(id).anchor, None, "{id} is canvas-bound");
        assert_eq!(app.rect(id), Rect::new(seen, 160.0, 100.0, 100.0), "{id}");
    }
    let comment = &app.document().annotations()[0];
    assert_eq!(comment.page_anchor, None, "the comment is freed too");
    let reopened = page_effects(app.undo());
    assert!(
        !app.app().can_undo(),
        "the delete and the freeing are one step"
    );
    assert_eq!(
        reopened,
        [Effect::CreatePage {
            page: EntityId::from("p1"),
            url: "https://example.com/p1".to_owned(),
            viewport: CssSize::new(400, 300)
        }]
    );
    assert_eq!(ids(&app), ["p1", "note", "box", "c"]);
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn backspace_in_an_entered_page_goes_to_the_page() {
    let mut app = TestApp::with_pages(1);
    app.click((200.0, 200.0))
        .click((200.0, 200.0))
        .take_effects();
    app.key(Key::Backspace);
    let keys = (app.take_effects().iter())
        .filter(|effect| {
            matches!(
                effect,
                Effect::ForwardInput {
                    event: InputEvent::Key(_),
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        (keys, ids(&app), app.app().can_undo()),
        (2, vec!["p1"], false)
    );
}

// Duplicate.

#[test]
fn command_d_copies_the_selection_to_its_right_and_selects_the_copy() {
    let mut app = TestApp::with_entities([text("a", A)]);
    app.select(&["a"]).chord(CMD, Key::Char('d'));
    // 80 clear of the original, on the grid.
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"text","x":100,"y":100,"width":200,"height":100,"text":"a"}
      {"id":"e220a8397b1dcdaf","type":"text","x":380,"y":100,"width":200,"height":100,"text":"a"}
    edges:
    specular: {"entityOrder":["a","e220a8397b1dcdaf"]}
    "#);
    assert_eq!(app.selected(), Some("e220a8397b1dcdaf"));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_duplicate_goes_below_when_the_right_is_taken_and_further_when_both_are() {
    let mut app = TestApp::with_entities([text("a", A), text("b", B)]);
    app.select(&["a"]).act(Action::Duplicate);
    let copy = app.selected().unwrap().to_owned();
    assert_eq!(app.rect(&copy), Rect::new(100.0, 280.0, 200.0, 100.0));
    app.assert_undo_returns_to_start();

    let below = Rect::new(100.0, 280.0, 200.0, 100.0);
    let mut app = TestApp::with_entities([text("a", A), text("b", B), text("c", below)]);
    app.select(&["a"]).act(Action::Duplicate);
    let copy = app.selected().unwrap().to_owned();
    assert_eq!(app.rect(&copy), Rect::new(680.0, 100.0, 200.0, 100.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn delete_duplicate_and_nudge_with_nothing_selected_record_nothing() {
    let mut app = linked();
    app.key(Key::Backspace)
        .chord(CMD, Key::Char('d'))
        .key(Key::ArrowRight);
    assert!(!app.app().can_undo());
}

#[test]
fn duplicating_several_keeps_their_layout_and_the_edges_between_them() {
    let mut app = linked();
    app.select(&["a", "b"]).act(Action::Duplicate);
    // `ac` has an end outside the selection, so it is not copied.
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"text","x":100,"y":100,"width":200,"height":100,"text":"a"}
      {"id":"b","type":"text","x":400,"y":100,"width":200,"height":100,"text":"b"}
      {"id":"c","type":"shape","x":100,"y":400,"width":100,"height":100,"shapeKind":"rectangle","text":""}
      {"id":"e220a8397b1dcdaf","type":"text","x":680,"y":100,"width":200,"height":100,"text":"a"}
      {"id":"6e789e6aa1b965f4","type":"text","x":980,"y":100,"width":200,"height":100,"text":"b"}
    edges:
      {"id":"ab","fromNode":"a","toNode":"b"}
      {"id":"ac","fromNode":"a","toNode":"c"}
      {"id":"06c45d188009454f","fromNode":"e220a8397b1dcdaf","toNode":"6e789e6aa1b965f4"}
    specular: {"entityOrder":["a","b","c","ab","ac","e220a8397b1dcdaf","6e789e6aa1b965f4","06c45d188009454f"]}
    "#);
    assert_eq!(app.selected_ids().len(), 2);
    app.undo();
    assert!(!app.app().can_undo(), "the duplicate was one step");
    app.redo().assert_undo_returns_to_start();
}

// Nudge.

#[test]
fn an_arrow_key_moves_the_selection_five_units() {
    let mut app = linked();
    app.select(&["a", "c"]);
    let right = app.key(Key::ArrowRight).rect("a");
    let down = app.key(Key::ArrowDown).rect("a");
    let left = app.key(Key::ArrowLeft).key(Key::ArrowLeft).rect("a");
    let up = app.key(Key::ArrowUp).key(Key::ArrowUp).rect("a");
    assert_eq!(
        (right.x, down.y, left.x, up.y, app.rect("c"), app.rect("b")),
        (
            105.0,
            105.0,
            95.0,
            95.0,
            Rect::new(95.0, 395.0, 100.0, 100.0),
            B
        )
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_nudge_is_exact_and_does_not_pull_onto_the_grid() {
    let off_grid = Rect::new(103.0, 101.0, 200.0, 100.0);
    let mut app = TestApp::with_entities([text("a", off_grid)]);
    app.select(&["a"])
        .key(Key::ArrowRight)
        .chord(SHIFT, Key::ArrowDown);
    assert_eq!(app.rect("a"), Rect::new(108.0, 121.0, 200.0, 100.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_nudged_group_takes_its_members_and_a_drawing_its_points() {
    let stroke = Stroke {
        id: "ink".to_owned(),
        color: Color::Neutral,
        width: 2.0,
        points: vec![Point::new(120.0, 120.0), Point::new(160.0, 150.0)],
        brush: None,
        extra: JsonMap::new(),
    };
    let ink = Entity {
        kind: Kind::Drawing(Drawing {
            strokes: vec![stroke],
        }),
        ..drawing("d", Rect::new(120.0, 120.0, 40.0, 30.0))
    };
    let mut app = TestApp::with_entities([
        group("g", Rect::new(100.0, 100.0, 300.0, 200.0)),
        inside("g", ink),
    ]);
    app.select(&["g"]).act(Action::Nudge { dx: 7.0, dy: -3.0 });
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"g","type":"group","x":107,"y":97,"width":300,"height":200}
      {"id":"d","type":"drawing","x":127,"y":117,"width":40,"height":30,"strokes":[{"id":"ink","color":"neutral","width":2,"points":[{"x":127,"y":117},{"x":167,"y":147}]}],"parentGroupId":"g"}
    edges:
    specular: {"entityOrder":["g","d"]}
    "#);
    app.assert_undo_returns_to_start();
}
