//! Element attachment (ADR 0032): an item hooked to a page follows the
//! element it was placed over. The page is asked for that element after
//! every step that puts the item somewhere new, the answer is stamped on the
//! anchor with no undo step, and the item is then seen wherever the element
//! goes.
//!
//! The page `p1` is 400x300 at (100, 100), and its CSS pixels are canvas
//! units. A synthetic page's grid has a cell every 160 across and 48 down.

use glam::Vec2;
use specular_core::{CapturedElement, CssSize, ElementPlace, synthetic_capture};
use specular_doc::{
    AnchorElement, Color, Drawing, Entity, JsonMap, Kind, PageAnchor, Point, Rect, Stroke,
};
use specular_interact::{Effect, Key, MarkShape, PageNotice, Tool, seen, shown_rect};
use specular_testkit::{TestApp, drawing, page, shape};

const P1: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);
const HERO: &str = "#hero";

fn scrolled(app: &mut TestApp, y: f64) {
    app.page_reports("p1", PageNotice::Scrolled { x: 0.0, y });
}

fn place(x: f32, y: f32, fixed: bool) -> ElementPlace {
    ElementPlace {
        doc: Vec2::new(x, y),
        viewport_positioned: fixed,
    }
}

/// The page says where the element `selector` names is now.
fn moved(app: &mut TestApp, selector: &str, to: Option<ElementPlace>) {
    let places = vec![(selector.to_owned(), to)];
    app.page_reports("p1", PageNotice::ElementPlaces(places));
}

/// `entity` hooked to `p1` at no scroll and attached to [`HERO`], which sat
/// at (100, 100) in the document.
fn attached(entity: Entity, fixed: bool) -> Entity {
    Entity {
        anchor: Some(PageAnchor {
            page_url: Some("https://example.com/p1".to_owned()),
            scroll_x: Some(0.0),
            scroll_y: Some(0.0),
            element: Some(AnchorElement {
                selector: HERO.to_owned(),
                doc_x: 100.0,
                doc_y: 100.0,
                viewport_positioned: fixed.then_some(true),
                extra: JsonMap::new(),
            }),
            ..PageAnchor::new("p1".into())
        }),
        ..entity
    }
}

fn element_of(app: &TestApp, id: &str) -> Option<(String, f64, f64)> {
    let element = app.entity(id).anchor.as_ref()?.element.as_ref()?;
    Some((element.selector.clone(), element.doc_x, element.doc_y))
}

fn cell(column: u32, row: u32) -> String {
    format!("div.cell[data-col=\"{column}\"][data-row=\"{row}\"]")
}

#[test]
fn an_item_put_on_a_page_is_attached_to_the_element_under_it_with_no_undo_step() {
    let mut app = TestApp::with_pages(1);
    scrolled(&mut app, 100.0);
    app.tool(Tool::AddShape)
        .drag((200.0, 200.0), (300.0, 300.0));
    let id = app.selected().expect("placed").to_owned();
    // Its centre is 150 into the page each way, and the page is 100 down.
    let (asked, first, point) = app.capture_asked();
    assert_eq!((asked.as_str(), point), ("p1", Vec2::new(150.0, 250.0)));
    app.assert_undo_returns_to_start();

    // Moved 160 right, it is asked about again where it now is.
    app.take_effects();
    app.drag((250.0, 250.0), (410.0, 250.0));
    let (_, _, point) = app.capture_asked();
    assert_eq!(point, Vec2::new(310.0, 250.0));
    // An answer to the question the move replaced is dropped.
    let late = CapturedElement {
        selector: "#late".to_owned(),
        place: place(0.0, 0.0, false),
    };
    app.page_reports(
        "p1",
        PageNotice::ElementCaptured {
            request: first,
            element: Some(late),
        },
    );
    assert_eq!(element_of(&app, &id), None);

    app.answer_capture();
    assert_eq!(element_of(&app, &id), Some((cell(1, 5), 160.0, 240.0)));
    // It is written as Electron writes it: nested in the anchor, with the
    // rail flag left out unless the element is on one.
    let saved = app.document().to_canvas_value().expect("a canvas");
    let node = (saved["nodes"].as_array().expect("nodes").iter())
        .find(|node| node["id"] == id.as_str())
        .expect("the shape");
    assert_eq!(
        node["pageAnchor"]["element"],
        serde_json::json!({ "selector": cell(1, 5), "docX": 160, "docY": 240 })
    );
    // It is saved, and the page is told to report where the cell goes.
    let effects = app.take_effects();
    assert!(effects.contains(&Effect::Save));
    assert!(effects.contains(&Effect::TrackElements {
        page: "p1".into(),
        selectors: vec![cell(1, 5)],
    }));
    // The stamp is no step: the two undos are the move and the placement.
    app.undo().undo();
    assert!(!app.app().can_undo());
    assert_eq!(app.document().entities().count(), 1);
}

#[test]
fn an_attached_item_is_seen_where_its_element_went() {
    // (scroll, where the page says the element is, attached to a fixed
    // element, how far from its stored place the item is seen)
    let hero = |x, y| Some((HERO, place(x, y, false)));
    let rows = [
        // The element moved 60 down, and the item with it.
        (0.0, hero(100.0, 160.0), false, (0.0, 60.0)),
        // Scrolled 200 down and moved 30 down: 200 up and 30 down.
        (200.0, hero(100.0, 130.0), false, (0.0, -170.0)),
        (0.0, hero(140.0, 100.0), false, (40.0, 0.0)),
        // Another element says nothing about this one: it stays as stored.
        (
            0.0,
            Some(("#other", place(0.0, 999.0, false))),
            false,
            (0.0, 0.0),
        ),
        // On a fixed element nobody has found, it stays in the frame.
        (50.0, None, true, (0.0, 0.0)),
    ];
    let stored = Rect::new(200.0, 200.0, 100.0, 100.0);
    for (scroll, said, fixed, (dx, dy)) in rows {
        let mut app = TestApp::with_entities([page("p1", P1), attached(shape("s", stored), fixed)]);
        scrolled(&mut app, scroll);
        if let Some((selector, at)) = said {
            moved(&mut app, selector, Some(at));
        }
        assert_eq!(
            shown_rect(app.app(), app.entity("s")),
            Some(stored.translated(dx, dy)),
            "scroll {scroll}, {said:?}"
        );
        assert_eq!(app.rect("s"), stored, "the stored rect is the truth");
    }
}

#[test]
fn an_item_on_a_fixed_element_stays_with_it_through_a_scroll() {
    let stored = Rect::new(200.0, 200.0, 100.0, 100.0);
    let mut app = TestApp::with_entities([page("p1", P1), attached(shape("s", stored), true)]);
    moved(&mut app, HERO, Some(place(100.0, 100.0, true)));
    // A fixed element travels through the document with the scroll, so the
    // item stays in the frame before the page has said where it went.
    scrolled(&mut app, 80.0);
    assert_eq!(shown_rect(app.app(), app.entity("s")), Some(stored));
    // And when the page says, nothing moves.
    moved(&mut app, HERO, Some(place(100.0, 180.0, true)));
    assert_eq!(shown_rect(app.app(), app.entity("s")), Some(stored));
}

#[test]
fn a_drawings_ink_goes_where_its_element_went() {
    let stroke = Stroke {
        id: "ink".to_owned(),
        color: Color::Neutral,
        width: 2.0,
        points: vec![Point::new(210.0, 210.0), Point::new(290.0, 290.0)],
        brush: None,
        extra: JsonMap::new(),
    };
    let ink = Entity {
        kind: Kind::Drawing(Drawing {
            strokes: vec![stroke],
        }),
        ..drawing("d", Rect::new(200.0, 200.0, 100.0, 100.0))
    };
    let mut app = TestApp::with_entities([page("p1", P1), attached(ink, false)]);
    moved(&mut app, HERO, Some(place(100.0, 150.0, false)));
    let shown = seen(app.app(), app.entity("d")).expect("still on the page");
    let Kind::Drawing(ink) = &shown.entity.kind else {
        panic!("a drawing")
    };
    assert_eq!(ink.strokes[0].points[0], Point::new(210.0, 260.0));
}

#[test]
fn a_move_folds_the_elements_travel_into_the_rect_and_restamps_where_it_is() {
    let stored = Rect::new(200.0, 200.0, 100.0, 100.0);
    let mut app = TestApp::with_entities([page("p1", P1), attached(shape("s", stored), false)]);
    // The element is 40 down, so the shape is seen at y 240 to 340.
    moved(&mut app, HERO, Some(place(100.0, 140.0, false)));
    app.drag((250.0, 290.0), (290.0, 290.0));
    assert_eq!(app.rect("s"), Rect::new(240.0, 240.0, 100.0, 100.0));
    assert_eq!(element_of(&app, "s"), Some((HERO.to_owned(), 100.0, 140.0)));
    assert_eq!(
        shown_rect(app.app(), app.entity("s")),
        Some(app.rect("s")),
        "stored where it is seen"
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn an_undo_that_moves_an_attached_item_asks_what_it_is_over_again() {
    let hooked = Entity {
        anchor: Some(PageAnchor {
            scroll_x: Some(0.0),
            scroll_y: Some(0.0),
            ..PageAnchor::new("p1".into())
        }),
        ..shape("s", Rect::new(120.0, 200.0, 100.0, 100.0))
    };
    let mut app = TestApp::with_entities([page("p1", P1), hooked]);
    app.drag((170.0, 250.0), (330.0, 250.0)).answer_capture();
    assert_eq!(element_of(&app, "s"), Some((cell(1, 3), 160.0, 144.0)));
    app.take_effects();
    app.undo();
    // Back where it was, over the first column.
    let (_, _, point) = app.capture_asked();
    assert_eq!(point, Vec2::new(70.0, 150.0));
    app.answer_capture();
    assert_eq!(element_of(&app, "s"), Some((cell(0, 3), 0.0, 144.0)));
    assert!(!app.app().can_undo());
}

#[test]
fn a_region_comment_on_a_page_is_attached_once_and_follows_its_element() {
    let mut app = TestApp::with_pages(1);
    app.tool(Tool::Comment)
        .tick(86_400_000)
        .drag((150.0, 150.0), (250.0, 250.0))
        .answer_grab(&[1])
        .type_text("tighten")
        .key(Key::Enter);
    // The region is 50 to 150 of the document each way.
    let (_, request, point) = app.capture_asked();
    assert_eq!(point, Vec2::new(100.0, 100.0));
    // It is asked about once: a later step asks nothing more, answered or
    // not.
    app.take_effects();
    app.tool(Tool::AddShape)
        .drag((600.0, 600.0), (700.0, 700.0));
    let asked = |effect: &Effect| matches!(effect, Effect::CaptureElement { .. });
    assert!(!app.effects().iter().any(asked));

    let element = Some(synthetic_capture(CssSize::new(400, 300), point));
    app.page_reports("p1", PageNotice::ElementCaptured { request, element });
    let binding = app.document().annotations()[0].page_anchor.clone();
    let element = binding
        .and_then(|binding| binding.element)
        .expect("attached");
    assert_eq!(
        (element.selector.as_str(), element.doc_x, element.doc_y),
        (cell(0, 2).as_str(), 0.0, 96.0)
    );

    // The cell moves 30 down, and the region with it.
    moved(&mut app, &cell(0, 2), Some(place(0.0, 126.0, false)));
    let marks = app.app().comment_marks();
    let MarkShape::Region(frame) = marks[0].shape else {
        panic!("a region: {marks:?}")
    };
    assert_eq!(frame.min, Vec2::new(150.0, 180.0));

    app.undo().undo();
    assert!(!app.app().can_undo());
    assert_eq!(app.document().annotations().len(), 0);
}
