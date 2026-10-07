//! Selection, focus, forwarding, pan and zoom, keys, and page hosting.

use specular_core::{
    CssSize, ImeEvent, InputEvent, KeyEventKind, PixelRect, PointerEvent, WheelEvent,
};
use specular_doc::Rect;

use super::*;
use crate::{Cursor, Focus, PageNotice, WheelInput};

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
        position: Vec2::new(at.0, at.1),
        modifiers: Modifiers::default(),
    })
}

fn wheel(app: &mut App, delta: (f32, f32), modifiers: Modifiers) -> Vec<Effect> {
    update(
        app,
        Event::Wheel(WheelInput {
            delta: Vec2::new(delta.0, delta.1),
            modifiers,
        }),
    )
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
    let mut app = App::new(0);
    let effects = update(
        &mut app,
        Event::DocumentOpened(Box::new(document(two_pages()))),
    );
    assert_eq!(
        effects,
        [
            Effect::CreatePage {
                page: id("p1"),
                url: "https://example.com/p1".to_owned(),
                viewport: CssSize::new(400, 300)
            },
            Effect::CreatePage {
                page: id("p2"),
                url: "https://example.com/p2".to_owned(),
                viewport: CssSize::new(400, 300)
            },
        ]
    );
}

#[test]
fn opening_another_document_closes_the_pages_it_does_not_hold() {
    let mut app = app();
    select(&mut app, "p1");
    press(&mut app, (800.0, 200.0));
    let kept = vec![page("p1", Rect::new(0.0, 0.0, 800.0, 600.0))];
    let effects = update(&mut app, Event::DocumentOpened(Box::new(document(kept))));
    assert_eq!(
        effects,
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
    assert_eq!((selected(&app), app.can_undo()), (None, false));
}

#[test]
fn clicking_a_page_selects_focuses_and_forwards() {
    let mut app = app();
    let effects = press(&mut app, (200.0, 200.0));
    assert_eq!(
        effects,
        [
            Effect::FocusPage(Some(id("p1"))),
            Effect::SetImeAllowed(true),
            Effect::ForwardInput {
                page: id("p1"),
                event: pointer_event(LEFT_DOWN, (100.0, 100.0))
            },
        ]
    );
    assert_eq!(
        (selected(&app), &app.session().focus),
        (Some(id("p1")), &Focus::Page(id("p1")))
    );
}

#[test]
fn clicking_empty_canvas_clears_the_selection_and_focus() {
    let mut app = app();
    press(&mut app, (200.0, 200.0));
    let effects = press(&mut app, (600.0, 600.0));
    assert_eq!(
        (effects, selected(&app)),
        (
            vec![Effect::FocusPage(None), Effect::SetImeAllowed(false)],
            None
        )
    );
}

#[test]
fn the_release_goes_to_the_page_that_got_the_press() {
    let mut app = app();
    press(&mut app, (200.0, 200.0));
    // Released over p2, 610 px right of p1's left edge.
    let effects = release(&mut app, (710.0, 150.0));
    assert_eq!(
        forwarded(&effects),
        [("p1", &pointer_event(LEFT_UP, (610.0, 50.0)))]
    );
}

#[test]
fn a_right_press_is_forwarded_without_focusing() {
    let mut app = app();
    let kind = PointerEventKind::Down {
        button: PointerButton::Right,
        click_count: 1,
    };
    let effects = pointer(&mut app, kind, (200.0, 200.0), Modifiers::default());
    assert_eq!(
        (
            forwarded(&effects).len(),
            effects.len(),
            &app.session().focus
        ),
        (1, 1, &Focus::Canvas)
    );
}

#[test]
fn moving_between_pages_tells_the_one_the_pointer_left() {
    let mut app = app();
    drag_to(&mut app, (200.0, 200.0));
    let effects = drag_to(&mut app, (800.0, 200.0));
    assert_eq!(
        forwarded(&effects),
        [
            (
                "p1",
                &pointer_event(PointerEventKind::Leave, (100.0, 100.0))
            ),
            ("p2", &pointer_event(PointerEventKind::Move, (100.0, 100.0))),
        ]
    );
    assert_eq!(app.session().hover, Some(id("p2")));
}

#[test]
fn the_pointer_leaving_the_window_ends_the_hover_unless_dragging() {
    let mut app = app();
    drag_to(&mut app, (200.0, 200.0));
    let left = pointer(
        &mut app,
        PointerEventKind::Leave,
        (0.0, 0.0),
        Modifiers::default(),
    );
    press_with(&mut app, (200.0, 150.0), ALT);
    pointer(
        &mut app,
        PointerEventKind::Leave,
        (0.0, 0.0),
        Modifiers::default(),
    );
    assert_eq!(
        (forwarded(&left).len(), app.session().pointer),
        (1, Some(Vec2::new(200.0, 150.0)))
    );
}

#[test]
fn pointer_positions_follow_the_page_scale_and_the_camera() {
    let mut app = app();
    act(
        &mut app,
        Action::SetCamera(Camera::new(Vec2::new(10.0, 20.0), 0.5)),
    );
    // Screen (110, 120) is canvas (200, 200): 100 px into p1 each way.
    let effects = drag_to(&mut app, (110.0, 120.0));
    assert_eq!(
        forwarded(&effects),
        [("p1", &pointer_event(PointerEventKind::Move, (100.0, 100.0)))]
    );
}

#[test]
fn wheel_over_the_focused_page_scrolls_it_in_css_pixels() {
    let mut app = app();
    set_zoom(&mut app, 0.5);
    press(&mut app, (100.0, 100.0));
    let effects = wheel(&mut app, (0.0, -30.0), Modifiers::default());
    assert_eq!(
        (effects, app.session().camera.pan),
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
fn wheel_over_an_unfocused_page_pans_the_canvas() {
    let mut app = app();
    drag_to(&mut app, (200.0, 200.0));
    let effects = wheel(&mut app, (12.0, -30.0), Modifiers::default());
    assert_eq!(
        (effects, app.session().camera.pan),
        (Vec::new(), Vec2::new(12.0, -30.0))
    );
}

#[test]
fn command_wheel_zooms_about_the_pointer_even_over_the_focused_page() {
    let mut app = app();
    press(&mut app, (200.0, 200.0));
    let effects = wheel(&mut app, (0.0, 100.0), CMD);
    let camera = app.session().camera;
    assert_eq!(effects, Vec::new());
    assert!((camera.zoom - 1.2).abs() < 1e-6);
    assert!(
        camera
            .world_to_screen(Vec2::new(200.0, 200.0))
            .abs_diff_eq(Vec2::new(200.0, 200.0), 1e-3)
    );
}

#[test]
fn pinch_zooms_about_the_viewport_centre_when_the_pointer_is_outside() {
    let mut app = app();
    update(&mut app, Event::ViewportResized(Vec2::new(1000.0, 800.0)));
    update(&mut app, Event::Pinch { delta: 0.5 });
    let camera = app.session().camera;
    assert!((camera.zoom - 1.5).abs() < 1e-6);
    assert!(
        camera
            .world_to_screen(Vec2::new(500.0, 400.0))
            .abs_diff_eq(Vec2::new(500.0, 400.0), 1e-3)
    );
}

#[test]
fn c_toggles_the_comment_tool_and_its_cursor() {
    let mut app = app();
    let on = key(&mut app, Key::Char('c'));
    let off = key(&mut app, Key::Char('c'));
    assert_eq!(
        (on, off, app.session().tool),
        (
            vec![Effect::SetCursor(Cursor::Crosshair)],
            vec![Effect::SetCursor(Cursor::Default)],
            Tool::Select
        )
    );
}

#[test]
fn keys_go_to_the_focused_page_instead_of_the_bindings() {
    let mut app = app();
    press(&mut app, (200.0, 200.0));
    let typed = key(&mut app, Key::Char('c'));
    let undo = key_with(&mut app, Key::Char('z'), true, CMD);
    let kinds: Vec<_> = forwarded(&typed)
        .into_iter()
        .chain(forwarded(&undo))
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
fn keys_with_no_page_focused_go_nowhere() {
    let mut app = app();
    assert_eq!(key(&mut app, Key::Char('x')), Vec::new());
}

#[test]
fn escape_leaves_the_tool_and_the_page_and_is_never_forwarded() {
    let mut app = app();
    press(&mut app, (200.0, 200.0));
    let down = key(&mut app, Key::Escape);
    let up = key_with(&mut app, Key::Escape, false, Modifiers::default());
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
    let mut app = armed();
    let effects = key(&mut app, Key::Escape);
    assert_eq!(
        (effects, app.session().tool),
        (vec![Effect::SetCursor(Cursor::Default)], Tool::Select)
    );
}

#[test]
fn ime_text_goes_to_the_focused_page_only() {
    let mut app = app();
    let commit = || {
        Event::Ime(ImeEvent::Commit {
            text: "ok".to_owned(),
            replacement: None,
        })
    };
    let unfocused = update(&mut app, commit());
    press(&mut app, (200.0, 200.0));
    let focused = update(&mut app, commit());
    assert_eq!((unfocused.len(), forwarded(&focused).len()), (0, 1));
}

#[test]
fn the_candidate_window_follows_the_focused_pages_composition() {
    let mut app = app();
    press(&mut app, (200.0, 200.0));
    set_zoom(&mut app, 0.5);
    let effects = update(
        &mut app,
        Event::Page {
            page: id("p1"),
            notice: PageNotice::ImeCompositionBounds(Some(PixelRect::new(20, 40, 60, 16))),
        },
    );
    assert_eq!(
        effects,
        [Effect::SetImeCursorArea {
            origin: Vec2::new(60.0, 70.0),
            size: Vec2::new(30.0, 8.0)
        }]
    );
}

#[test]
fn selecting_ids_that_name_nothing_selects_nothing() {
    let mut app = app();
    select(&mut app, "ghost");
    assert!(app.session().selection.is_empty());
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
