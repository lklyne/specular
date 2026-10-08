//! The select tool: click, Shift-click, marquee, entering a page (ADR 0022)
//! and what the selection means for a group (ADR 0034). Nothing here changes
//! the document.

use specular_core::{InputEvent, Modifiers, PointerButton, PointerEvent, PointerEventKind};
use specular_doc::{Entity, EntityId, PageAnchor, Rect};
use specular_interact::{Effect, Event, Focus, Gesture, MarqueeMode, PointerInput};
use specular_testkit::{
    CMD, SHIFT, TestApp, connected, document, group, inside, page, pages, shape, text,
};

/// A point on `p1`, on `p2`, and on empty canvas below both.
const ON_P1: (f32, f32) = (200.0, 200.0);
const ON_P2: (f32, f32) = (800.0, 200.0);
const EMPTY: (f32, f32) = (600.0, 600.0);

fn entering(page: &str) -> Vec<Effect> {
    vec![
        Effect::FocusPage(Some(EntityId::from(page))),
        Effect::SetImeAllowed(true),
    ]
}

fn leaving() -> Vec<Effect> {
    vec![Effect::FocusPage(None), Effect::SetImeAllowed(false)]
}

fn focus(app: &TestApp) -> Option<&str> {
    app.session().focus.page().map(EntityId::as_str)
}

/// The pointer events forwarded to pages: who got which kind.
fn forwarded(effects: &[Effect]) -> Vec<(&str, PointerEventKind)> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::ForwardInput {
                page,
                event: InputEvent::Pointer(PointerEvent { kind, .. }),
            } => Some((page.as_str(), *kind)),
            _ => None,
        })
        .collect()
}

const LEFT_DOWN: PointerEventKind = PointerEventKind::Down {
    button: PointerButton::Left,
    click_count: 1,
};
const LEFT_UP: PointerEventKind = PointerEventKind::Up {
    button: PointerButton::Left,
    click_count: 1,
};

// Pages: select first, interact second.

#[test]
fn the_first_click_on_a_page_selects_it_and_tells_the_page_nothing() {
    let mut app = TestApp::with_pages(2);
    app.click(ON_P1);
    assert_eq!(
        (app.take_effects(), app.selected(), &app.session().focus),
        (Vec::new(), Some("p1"), &Focus::Canvas)
    );
}

#[test]
fn a_click_on_the_selected_page_enters_it_without_forwarding_that_click() {
    let mut app = TestApp::with_pages(2);
    app.click(ON_P1).click(ON_P1);
    assert_eq!(
        (app.take_effects(), app.selected(), focus(&app)),
        (entering("p1"), Some("p1"), Some("p1"))
    );
}

#[test]
fn clicks_on_the_entered_page_go_to_the_page() {
    let mut app = TestApp::with_pages(2);
    app.click(ON_P1).click(ON_P1).take_effects();
    app.click(ON_P1);
    assert_eq!(
        forwarded(app.effects()),
        [("p1", LEFT_DOWN), ("p1", LEFT_UP)]
    );
}

#[test]
fn a_double_click_enters_a_page_that_was_not_selected() {
    let mut app = TestApp::with_pages(2);
    // Only the second press of the pair arrives: the count alone enters.
    for kind in [
        PointerEventKind::Down {
            button: PointerButton::Left,
            click_count: 2,
        },
        PointerEventKind::Up {
            button: PointerButton::Left,
            click_count: 2,
        },
    ] {
        app.send(Event::Pointer(PointerInput {
            kind,
            screen: ON_P1.into(),
            modifiers: Modifiers::default(),
        }));
    }
    assert_eq!(
        (app.take_effects(), focus(&app)),
        (entering("p1"), Some("p1"))
    );
}

#[test]
fn selecting_another_page_leaves_the_entered_one() {
    let mut app = TestApp::with_pages(2);
    app.click(ON_P1).click(ON_P1).take_effects();
    app.click(ON_P2);
    assert_eq!(
        (app.take_effects(), app.selected(), focus(&app)),
        (leaving(), Some("p2"), None)
    );
}

#[test]
fn an_item_over_the_entered_page_still_takes_the_press() {
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        shape("s1", Rect::new(150.0, 150.0, 50.0, 50.0)),
    ]);
    app.click((300.0, 300.0))
        .click((300.0, 300.0))
        .take_effects();
    app.click((170.0, 170.0));
    assert_eq!(
        (app.take_effects(), app.selected()),
        (leaving(), Some("s1"))
    );
}

// Click and Shift-click.

#[test]
fn shift_click_adds_to_the_selection_and_takes_away_from_it() {
    let mut app = TestApp::with_pages(2);
    app.click(ON_P1).hold(SHIFT).click(ON_P2);
    let both = app.selected_ids().join(",");
    app.click(ON_P1);
    assert_eq!((both.as_str(), app.selected_ids()), ("p1,p2", vec!["p2"]));
}

#[test]
fn shift_click_on_the_selected_page_deselects_instead_of_entering() {
    let mut app = TestApp::with_pages(2);
    app.click(ON_P1).hold(SHIFT).click(ON_P1);
    assert_eq!(
        (app.take_effects(), app.selection().is_empty()),
        (Vec::new(), true)
    );
}

#[test]
fn a_click_on_empty_canvas_clears_the_selection_but_a_shift_click_keeps_it() {
    let mut app = TestApp::with_pages(2);
    app.select(&["p1", "p2"]).hold(SHIFT).click(EMPTY).let_go();
    let shifted = app.selected_ids().join(",");
    app.click(EMPTY);
    assert_eq!(
        (shifted.as_str(), app.selection().is_empty()),
        ("p1,p2", true)
    );
}

#[test]
fn a_click_on_an_edge_selects_the_edge() {
    let entities = [
        text("t1", Rect::new(100.0, 100.0, 100.0, 100.0)),
        text("t2", Rect::new(500.0, 100.0, 100.0, 100.0)),
    ];
    let mut app = TestApp::from_document(connected(document(entities), "e1", "t1", "t2"));
    app.click((350.0, 150.0));
    let alone = app.selected_ids().join(",");
    app.hold(SHIFT).click((150.0, 150.0));
    assert_eq!(
        (alone.as_str(), app.selected_ids()),
        ("e1", vec!["e1", "t1"])
    );
}

// Marquee.

#[test]
fn a_drag_from_empty_canvas_selects_what_the_rect_touches() {
    let mut app = TestApp::with_pages(3);
    // From below-left of p1 up into p2: touches p1 and p2, not p3.
    app.drag((50.0, 450.0), (750.0, 350.0));
    let taken = (app.selected_ids().join(","), app.app().can_undo());
    // Rects that miss every page on one side each: left, above, between, below.
    let misses = [
        ((10.0, 150.0), (90.0, 350.0)),
        ((50.0, 10.0), (750.0, 90.0)),
        ((520.0, 150.0), (580.0, 350.0)),
        ((50.0, 420.0), (750.0, 480.0)),
    ]
    .map(|(from, to)| app.drag(from, to).selection().is_empty());
    assert_eq!((taken, misses), (("p1,p2".to_string(), false), [true; 4]));
}

#[test]
fn a_press_is_not_a_marquee_until_it_travels_four_pixels() {
    let mut app = TestApp::with_pages(2);
    app.press(EMPTY).drag_to((603.0, 603.0));
    let before = app.app().marquee();
    app.drag_to((603.0, 604.0));
    assert_eq!(
        (before, app.app().marquee()),
        (None, Some(Rect::new(600.0, 600.0, 3.0, 4.0)))
    );
}

#[test]
fn a_command_marquee_takes_only_what_it_encloses() {
    let mut app = TestApp::with_pages(2);
    // Encloses p1 (100..500 x 100..400) and cuts through p2.
    app.hold(CMD).press((50.0, 50.0)).drag_to((750.0, 450.0));
    let mode = match app.session().gesture {
        Some(Gesture::Marquee { mode, .. }) => Some(mode),
        _ => None,
    };
    app.release();
    assert_eq!(
        (mode, app.selected_ids()),
        (Some(MarqueeMode::Contain), vec!["p1"])
    );
}

#[test]
fn a_shift_marquee_toggles_what_it_touches() {
    let mut app = TestApp::with_pages(3);
    app.select(&["p1", "p3"])
        .hold(SHIFT)
        .drag((50.0, 450.0), (750.0, 350.0));
    assert_eq!(app.selected_ids(), ["p3", "p2"]);
}

#[test]
fn a_marquee_takes_the_edges_it_crosses() {
    // The edge runs between the facing sides, (200, 150) to (500, 150).
    let entities = [
        text("t1", Rect::new(100.0, 100.0, 100.0, 100.0)),
        text("t2", Rect::new(500.0, 100.0, 100.0, 100.0)),
    ];
    let mut app = TestApp::from_document(connected(document(entities), "e1", "t1", "t2"));
    app.drag((300.0, 50.0), (400.0, 250.0));
    let crossed = app.selected_ids().join(",");
    // A rect beside the line takes nothing.
    app.drag((300.0, 300.0), (400.0, 400.0));
    let beside = app.selection().is_empty();
    // Enclosing only its middle is not enough with Command held, nor is
    // enclosing one end.
    app.click(EMPTY_FAR)
        .hold(CMD)
        .drag((300.0, 50.0), (400.0, 250.0));
    let middle = app.selection().is_empty();
    app.drag((50.0, 50.0), (250.0, 250.0));
    let one_end = app.selected_ids().join(",");
    // Both ends enclosed takes it. Command also toggles, so start clear.
    app.let_go().click(EMPTY_FAR).hold(CMD);
    app.drag((50.0, 50.0), (650.0, 250.0));
    let both = app.selected_ids().join(",");
    assert_eq!(
        (
            crossed.as_str(),
            beside,
            middle,
            one_end.as_str(),
            both.as_str()
        ),
        ("e1", true, true, "t1", "t1,t2,e1")
    );
}

// Groups. `g` spans (100, 100) to (700, 600) and holds `a` and `b`; `out`
// sits to its right.

fn grouped() -> Vec<Entity> {
    vec![
        group("g", Rect::new(100.0, 100.0, 600.0, 500.0)),
        inside("g", text("a", Rect::new(150.0, 150.0, 100.0, 40.0))),
        inside("g", text("b", Rect::new(150.0, 300.0, 100.0, 40.0))),
        text("out", Rect::new(800.0, 150.0, 100.0, 40.0)),
    ]
}

#[test]
fn a_click_inside_a_group_or_on_its_border_selects_the_group() {
    let mut app = TestApp::with_entities(grouped());
    let interior = app.click((500.0, 500.0)).selected_ids().join(",");
    let border = app
        .click(EMPTY_FAR)
        .click((103.0, 400.0))
        .selected_ids()
        .join(",");
    assert_eq!((interior.as_str(), border.as_str()), ("g", "g"));
}

const EMPTY_FAR: (f32, f32) = (1500.0, 900.0);

#[test]
fn a_marquee_that_encloses_a_group_takes_it_as_one() {
    let mut app = TestApp::with_entities(grouped());
    app.drag((50.0, 50.0), (750.0, 650.0));
    assert_eq!(app.selected_ids(), ["g"]);
}

#[test]
fn a_marquee_over_a_member_and_something_outside_takes_the_whole_group() {
    let mut app = TestApp::with_entities(grouped());
    // Starts on empty canvas right of `out`, ends over `a`.
    app.drag((950.0, 130.0), (240.0, 200.0));
    assert_eq!(app.selected_ids(), ["g", "out"]);
}

#[test]
fn nested_groups_resolve_to_the_group_directly_in_the_scope() {
    let mut app = TestApp::with_entities([
        group("outer", Rect::new(0.0, 0.0, 1000.0, 800.0)),
        inside(
            "outer",
            group("inner", Rect::new(100.0, 100.0, 300.0, 300.0)),
        ),
        inside("inner", text("deep", Rect::new(150.0, 150.0, 100.0, 40.0))),
        inside("outer", text("side", Rect::new(600.0, 150.0, 100.0, 40.0))),
    ]);
    // Inside `outer`, over `deep` and `side`: the scope is `outer`, so `deep`
    // becomes `inner`.
    app.hold(CMD)
        .press((950.0, 20.0))
        .let_go()
        .drag_to((240.0, 180.0))
        .release();
    assert_eq!(app.selected_ids(), ["inner", "side"]);
}

#[test]
fn a_selected_group_stands_for_everything_in_it() {
    let mut app = TestApp::with_entities(grouped());
    app.select(&["g", "out"]);
    let scope = app.app().selection_scope();
    let ids = |ids: &[EntityId]| {
        ids.iter()
            .map(EntityId::as_str)
            .collect::<Vec<_>>()
            .join(",")
    };
    assert_eq!(
        (
            ids(&scope.members).as_str(),
            ids(&scope.operands).as_str(),
            scope.bounds
        ),
        (
            "g,out",
            "g,a,b,out",
            Some(Rect::new(100.0, 100.0, 800.0, 500.0))
        )
    );
}

#[test]
fn what_is_hooked_to_a_selected_page_moves_with_it() {
    let note = Entity {
        anchor: Some(PageAnchor::new(EntityId::from("p1"))),
        ..text("note", Rect::new(450.0, 50.0, 100.0, 100.0))
    };
    let mut app = TestApp::with_entities(pages(2).into_iter().chain([note]));
    app.select(&["p1"]);
    let scope = app.app().selection_scope();
    assert_eq!(
        (
            scope.operands.len(),
            scope.bounds,
            scope.holds(&EntityId::from("note"))
        ),
        (2, Some(Rect::new(100.0, 50.0, 450.0, 350.0)), true)
    );
}
