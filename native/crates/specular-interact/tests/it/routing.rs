//! Page hosting, forwarding to the entered page, pan and zoom, and keys.

use glam::Vec2;
use specular_core::{
    CssSize, ImeEvent, InputEvent, KeyEventKind, Modifiers, PixelRect, PointerButton, PointerEvent,
    PointerEventKind, WheelEvent,
};
use specular_doc::{EntityId, Rect};
use specular_interact::{Effect, Event, Focus, Key, PageNotice, Tool};
use specular_testkit::{CMD, TestApp, document, page, pages};

fn id(id: &str) -> EntityId {
    EntityId::from(id)
}

/// Two pages with `page` entered: selected, then clicked again.
fn entered(page: &str) -> TestApp {
    let at = if page == "p1" {
        (200.0, 200.0)
    } else {
        (800.0, 200.0)
    };
    let mut app = TestApp::with_pages(2);
    app.click(at).click(at).take_effects();
    assert_eq!(app.session().focus, Focus::Page(id(page)));
    app
}

fn forwarded(effects: &[Effect]) -> Vec<(&str, &InputEvent)> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::ForwardInput { page, event } => Some((page.as_str(), event)),
            _ => None,
        })
        .collect()
}

fn pointer_event(kind: PointerEventKind, at: (f32, f32)) -> InputEvent {
    InputEvent::Pointer(PointerEvent {
        kind,
        position: Vec2::from(at),
        modifiers: Modifiers::default(),
    })
}

const LEFT_DOWN: PointerEventKind = PointerEventKind::Down {
    button: PointerButton::Left,
    click_count: 1,
};
const LEFT_UP: PointerEventKind = PointerEventKind::Up {
    button: PointerButton::Left,
    click_count: 1,
};

#[test]
fn opening_a_document_hosts_its_pages_in_stack_order() {
    let mut app = TestApp::empty();
    app.open(document(pages(2)));
    let create = |page: &str| Effect::CreatePage {
        page: id(page),
        url: format!("https://example.com/{page}"),
        viewport: CssSize::new(400, 300),
    };
    assert_eq!(app.take_effects(), [create("p1"), create("p2")]);
}

#[test]
fn opening_another_document_closes_the_pages_it_does_not_hold() {
    let mut app = TestApp::with_pages(2);
    // An undo step the other document must not inherit.
    app.drag((300.0, 300.0), (300.0, 340.0));
    assert!(app.app().can_undo());
    app.click((800.0, 200.0))
        .click((800.0, 200.0))
        .take_effects();
    assert_eq!(app.session().focus, Focus::Page(id("p2")));
    app.open(document([page("p1", Rect::new(0.0, 0.0, 800.0, 600.0))]));
    assert_eq!(
        app.take_effects(),
        [
            Effect::FocusPage(None),
            Effect::SetImeAllowed(false),
            Effect::ClosePage(id("p2")),
            Effect::SetPageViewport {
                page: id("p1"),
                viewport: CssSize::new(800, 600)
            },
        ]
    );
    assert_eq!((app.selected(), app.app().can_undo()), (None, false));
}

#[test]
fn a_press_on_the_entered_page_arrives_in_its_pixels() {
    let mut app = entered("p1");
    app.press((200.0, 250.0));
    assert_eq!(
        forwarded(app.effects()),
        [("p1", &pointer_event(LEFT_DOWN, (100.0, 150.0)))]
    );
}

#[test]
fn a_drag_that_starts_in_the_page_stays_with_it_until_the_release() {
    let mut app = entered("p1");
    // Dragged out over p2, 610 px right of p1's left edge.
    app.press((200.0, 200.0)).take_effects();
    let moved = app.drag_to((710.0, 150.0)).take_effects();
    let released = app.release().take_effects();
    assert_eq!(
        (forwarded(&moved), forwarded(&released)),
        (
            vec![("p1", &pointer_event(PointerEventKind::Move, (610.0, 50.0)))],
            vec![("p1", &pointer_event(LEFT_UP, (610.0, 50.0)))]
        )
    );
}

#[test]
fn hover_follows_the_pointer_over_any_kind_and_ends_off_the_window() {
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        specular_testkit::shape("s1", Rect::new(600.0, 100.0, 100.0, 100.0)),
    ]);
    let hover = |app: &TestApp| {
        app.session()
            .hover
            .as_ref()
            .map(|id| id.as_str().to_owned())
    };
    let over_page = hover(app.pointer_move((200.0, 200.0)));
    let over_shape = hover(app.pointer_move((650.0, 150.0)));
    let over_nothing = hover(app.pointer_move((900.0, 900.0)));
    let outside = hover(app.pointer_move((650.0, 150.0)).pointer_leave());
    assert_eq!(
        (
            over_page.as_deref(),
            over_shape.as_deref(),
            over_nothing,
            outside
        ),
        (Some("p1"), Some("s1"), None, None)
    );
}

#[test]
fn wheel_over_the_entered_page_scrolls_it_in_css_pixels() {
    let mut app = entered("p1");
    app.zoom(0.5).pointer_move((100.0, 100.0)).take_effects();
    app.wheel((0.0, -30.0));
    assert_eq!(
        (app.take_effects(), app.session().camera.pan),
        (
            vec![Effect::ForwardInput {
                page: id("p1"),
                event: InputEvent::Wheel(WheelEvent {
                    position: Vec2::new(100.0, 100.0),
                    delta: Vec2::new(0.0, -60.0),
                    modifiers: Modifiers::default()
                })
            }],
            Vec2::ZERO
        )
    );
}

#[test]
fn a_wheel_over_a_page_mid_resize_scrolls_by_the_css_pixels_it_is_still_laid_out_at() {
    let mut app = entered("p1");
    // The corner is dragged in to (300, 200): the page is 320 by 200 canvas
    // units over a 400 by 300 px viewport until the release lays it out
    // again, and the pointer is still over it.
    app.press((500.0, 400.0)).drag_to((300.0, 200.0));
    app.take_effects();
    app.wheel((0.0, -30.0));
    assert_eq!(
        app.take_effects(),
        [Effect::ForwardInput {
            page: id("p1"),
            event: InputEvent::Wheel(WheelEvent {
                position: Vec2::new(250.0, 150.0),
                delta: Vec2::new(0.0, -45.0),
                modifiers: Modifiers::default()
            })
        }]
    );
}

#[test]
fn command_wheel_zooms_about_the_pointer_even_over_the_entered_page() {
    let mut app = entered("p1");
    app.hold(CMD).wheel((0.0, 100.0));
    let camera = app.session().camera;
    assert_eq!(app.take_effects(), Vec::new());
    assert!((camera.zoom - 1.2).abs() < 1e-6);
    assert!(
        camera
            .world_to_screen(Vec2::new(200.0, 200.0))
            .abs_diff_eq(Vec2::new(200.0, 200.0), 1e-3)
    );
}

#[test]
fn pinch_zooms_about_the_viewport_centre_when_the_pointer_is_outside() {
    let mut app = TestApp::with_pages(2);
    app.viewport((1000.0, 800.0)).pinch(0.5);
    let camera = app.session().camera;
    assert!((camera.zoom - 1.5).abs() < 1e-6);
    assert!(
        camera
            .world_to_screen(Vec2::new(500.0, 400.0))
            .abs_diff_eq(Vec2::new(500.0, 400.0), 1e-3)
    );
    // With the pointer in the window the pinch is about it instead.
    let mut app = TestApp::with_pages(2);
    app.viewport((1000.0, 800.0))
        .pointer_move((200.0, 100.0))
        .pinch(0.5);
    let camera = app.session().camera;
    assert!(
        camera
            .world_to_screen(Vec2::new(200.0, 100.0))
            .abs_diff_eq(Vec2::new(200.0, 100.0), 1e-3)
    );
}

#[test]
fn keys_go_to_the_entered_page_instead_of_the_bindings() {
    let mut app = entered("p1");
    app.key_down(Key::Char('c'))
        .hold(CMD)
        .key_down(Key::Char('z'));
    let kinds: Vec<_> = forwarded(app.effects())
        .into_iter()
        .filter_map(|(_, event)| match event {
            InputEvent::Key(key) => Some(key.kind),
            _ => None,
        })
        .collect();
    assert_eq!(
        (kinds, app.session().tool),
        (
            vec![
                KeyEventKind::RawDown,
                KeyEventKind::Char,
                KeyEventKind::RawDown
            ],
            Tool::Select
        )
    );
}

/// CEF on macOS reads a key event with no character as a change of
/// modifiers, and a change of modifiers on a key that is not one as a press.
/// So a release without its character was a second press: one Backspace
/// deleted twice, and no key ever came up.
#[test]
fn a_key_reaches_the_entered_page_as_one_press_and_one_release_with_its_character() {
    let cases = [
        (Key::Backspace, 51, '\u{7f}'),
        (Key::ArrowLeft, 123, '\u{f702}'),
        (Key::Enter, 36, '\r'),
        (Key::Char('a'), 0, 'a'),
    ];
    for (key, key_code, character) in cases {
        let mut app = entered("p1");
        app.key(key);
        let events: Vec<_> = forwarded(app.effects())
            .into_iter()
            .filter_map(|(_, event)| match event {
                InputEvent::Key(key) if key.kind != KeyEventKind::Char => {
                    Some((key.kind, key.native_key_code, key.character))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            events,
            [
                (KeyEventKind::RawDown, key_code, Some(character)),
                (KeyEventKind::Up, key_code, Some(character)),
            ],
            "{key:?}"
        );
    }
}

#[test]
fn ime_text_goes_to_the_entered_page_only() {
    let commit = || {
        Event::Ime(ImeEvent::Commit {
            text: "ok".to_owned(),
            replacement: None,
        })
    };
    let mut selected = TestApp::with_pages(2);
    selected.click((200.0, 200.0)).send(commit());
    let mut app = entered("p1");
    app.send(commit());
    assert_eq!(
        (selected.effects().len(), forwarded(app.effects()).len()),
        (0, 1)
    );
}

#[test]
fn the_candidate_window_follows_the_entered_pages_composition() {
    let mut app = entered("p1");
    app.zoom(0.5).send(Event::Page {
        page: id("p1"),
        notice: PageNotice::ImeCompositionBounds(Some(PixelRect::new(20, 40, 60, 16))),
    });
    assert_eq!(
        app.take_effects(),
        [Effect::SetImeCursorArea {
            origin: Vec2::new(60.0, 70.0),
            size: Vec2::new(30.0, 8.0)
        }]
    );
}

#[test]
fn the_candidate_window_scales_by_the_page_layout_it_has_mid_resize() {
    let mut app = entered("p1");
    // Dragged out to 600 by 600 over a 400 by 300 viewport: 1.5 and 2 canvas
    // units per CSS pixel until the release.
    app.press((500.0, 400.0)).drag_to((700.0, 700.0));
    app.take_effects();
    app.send(Event::Page {
        page: id("p1"),
        notice: PageNotice::ImeCompositionBounds(Some(PixelRect::new(20, 40, 60, 16))),
    });
    assert_eq!(
        app.take_effects(),
        [Effect::SetImeCursorArea {
            origin: Vec2::new(130.0, 180.0),
            size: Vec2::new(90.0, 32.0)
        }]
    );
}

#[test]
fn a_composition_in_a_page_that_is_not_entered_leaves_the_candidate_window_alone() {
    let mut app = TestApp::with_pages(2);
    app.click((200.0, 200.0)).take_effects();
    app.send(Event::Page {
        page: id("p1"),
        notice: PageNotice::ImeCompositionBounds(Some(PixelRect::new(20, 40, 60, 16))),
    });
    assert_eq!(app.take_effects(), Vec::new());
}
