//! Page hosting, forwarding to the entered page, pan and zoom, and keys.

use glam::Vec2;
use specular_core::{
    Camera, CssSize, ImeEvent, InputEvent, KeyEventKind, Modifiers, PixelRect, PointerButton,
    PointerEvent, PointerEventKind, WheelEvent,
};
use specular_doc::{EntityId, Rect};
use specular_interact::{Action, Cursor, Effect, Event, Focus, Key, PageNotice, Tool};
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
    let mut app = entered("p2");
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
fn clicking_empty_canvas_leaves_the_page_and_clears_the_selection() {
    let mut app = entered("p1");
    app.click((600.0, 600.0));
    assert_eq!(
        (app.take_effects(), app.selected()),
        (
            vec![Effect::FocusPage(None), Effect::SetImeAllowed(false)],
            None
        )
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
fn a_right_press_reaches_only_the_entered_page() {
    let mut app = entered("p1");
    let on_entered = app
        .press_button(PointerButton::Right, (200.0, 200.0))
        .take_effects();
    let on_other = app
        .press_button(PointerButton::Right, (800.0, 200.0))
        .take_effects();
    assert_eq!(
        (forwarded(&on_entered).len(), on_other, app.selected()),
        (1, Vec::new(), Some("p1"))
    );
}

#[test]
fn only_the_entered_page_hears_the_pointer_move() {
    let mut app = entered("p1");
    let over = app.pointer_move((200.0, 200.0)).take_effects();
    let away = app.pointer_move((800.0, 200.0)).take_effects();
    assert_eq!(
        (forwarded(&over), forwarded(&away)),
        (
            vec![("p1", &pointer_event(PointerEventKind::Move, (100.0, 100.0)))],
            vec![("p1", &pointer_event(PointerEventKind::Leave, (0.0, 0.0)))],
        )
    );
}

#[test]
fn a_page_that_is_only_selected_is_hovered_but_hears_nothing() {
    let mut app = TestApp::with_pages(2);
    app.click((200.0, 200.0)).pointer_move((250.0, 250.0));
    assert_eq!(
        (app.take_effects(), app.session().hover.as_ref()),
        (Vec::new(), Some(&id("p1")))
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
fn the_pointer_leaving_the_window_ends_the_hover_unless_dragging() {
    let mut app = entered("p1");
    app.pointer_move((200.0, 200.0)).take_effects();
    let left = app.pointer_leave().take_effects();
    app.press((600.0, 600.0)).pointer_leave();
    assert_eq!(
        (forwarded(&left).len(), app.session().pointer),
        (1, Some(Vec2::new(600.0, 600.0)))
    );
}

#[test]
fn pointer_positions_follow_the_page_scale_and_the_camera() {
    let mut app = entered("p1");
    app.act(Action::SetCamera(Camera::new(Vec2::new(10.0, 20.0), 0.5)));
    // Screen (110, 120) is canvas (200, 200): 100 px into p1 each way.
    app.pointer_move((110.0, 120.0));
    assert_eq!(
        forwarded(app.effects()),
        [("p1", &pointer_event(PointerEventKind::Move, (100.0, 100.0)))]
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
fn wheel_over_a_page_that_is_not_entered_pans_the_canvas() {
    let mut app = TestApp::with_pages(2);
    app.click((200.0, 200.0)).wheel((12.0, -30.0));
    assert_eq!(
        (app.take_effects(), app.session().camera.pan),
        (Vec::new(), Vec2::new(12.0, -30.0))
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
}

#[test]
fn c_arms_the_comment_tool_and_its_cursor_and_a_second_c_changes_nothing() {
    let mut app = TestApp::with_pages(2);
    let on = app.key(Key::Char('c')).take_effects();
    let again = app.key(Key::Char('c')).take_effects();
    assert_eq!(
        (on, again, app.session().tool),
        (
            vec![Effect::SetCursor(Cursor::Crosshair)],
            vec![],
            Tool::Comment
        )
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

#[test]
fn keys_go_nowhere_while_a_page_is_only_selected() {
    let mut app = TestApp::with_pages(2);
    app.click((200.0, 200.0)).key(Key::Char('x'));
    assert_eq!(app.take_effects(), Vec::new());
}

#[test]
fn escape_leaves_the_page_and_is_never_forwarded() {
    let mut app = entered("p1");
    let down = app.key_down(Key::Escape).take_effects();
    let up = app.key_up(Key::Escape).take_effects();
    assert_eq!(
        (down, up, &app.session().focus),
        (
            vec![Effect::FocusPage(None), Effect::SetImeAllowed(false)],
            Vec::new(),
            &Focus::Canvas
        )
    );
}

#[test]
fn escape_leaves_the_comment_tool() {
    let mut app = TestApp::with_pages(2);
    app.key(Key::Char('c')).take_effects();
    app.key(Key::Escape);
    assert_eq!(
        (app.take_effects(), app.session().tool),
        (vec![Effect::SetCursor(Cursor::Default)], Tool::Select)
    );
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
fn selecting_ids_that_name_nothing_selects_nothing() {
    let mut app = TestApp::with_pages(2);
    app.select(&["ghost"]);
    assert!(app.selection().is_empty());
}

#[test]
fn every_tool_but_select_shows_the_crosshair() {
    let cursors = Tool::ALL.map(Tool::cursor);
    assert_eq!(
        cursors
            .iter()
            .filter(|cursor| **cursor == Cursor::Default)
            .count(),
        1
    );
}
