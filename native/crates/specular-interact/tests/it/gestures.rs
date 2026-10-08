//! Move, resize and the comment tool's region drag, with undo, scripted
//! through `specular-testkit`.

use specular_core::CssSize;
use specular_doc::{AnnotationAnchor, EntityId, Rect, RegionAnchor};
use specular_interact::{Cursor, Effect, Gesture, Key, Tool, region_on_canvas};
use specular_testkit::{ALT, CMD, TestApp};

const P1: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);
/// The bottom-right corner of `p1`, where its resize handle sits.
const P1_CORNER: (f32, f32) = (500.0, 400.0);

/// Two pages with `p1` selected, so its handles exist.
fn p1_selected() -> TestApp {
    let mut app = TestApp::with_pages(2);
    app.select(&["p1"]);
    app
}

/// Two pages with the comment tool armed.
fn armed() -> TestApp {
    let mut app = TestApp::with_pages(2);
    app.key(Key::Char('c'));
    assert_eq!(app.session().tool, Tool::Comment);
    app.take_effects();
    app
}

#[test]
fn a_drag_on_a_page_selects_it_and_moves_it_by_the_pointer_delta() {
    let mut app = TestApp::with_pages(2);
    app.press((200.0, 150.0)).drag_to((260.0, 130.0));
    assert_eq!(
        (app.take_effects(), app.rect("p1"), app.selected()),
        (Vec::new(), Rect::new(160.0, 80.0, 400.0, 300.0), Some("p1"))
    );
}

#[test]
fn move_follows_the_camera_zoom() {
    let mut app = TestApp::with_pages(2);
    app.zoom(0.5).press((100.0, 75.0)).drag_to((110.0, 75.0));
    assert_eq!(app.rect("p1").x, 120.0);
}

#[test]
fn undoing_a_resize_lays_the_page_out_at_its_old_viewport() {
    let mut app = p1_selected();
    app.drag(P1_CORNER, (600.0, 460.0)).take_effects();
    let effects = app.chord(CMD, Key::Char('z')).take_effects();
    // The handle goes back with the corner, out from under the pointer.
    assert_eq!(
        (effects, app.rect("p1")),
        (
            vec![
                Effect::SetPageViewport {
                    page: EntityId::from("p1"),
                    viewport: CssSize::new(400, 300)
                },
                Effect::Save,
                Effect::SetCursor(Cursor::Default)
            ],
            P1
        )
    );
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn undo_is_ignored_while_a_drag_is_in_flight() {
    let mut app = TestApp::with_pages(2);
    app.drag((200.0, 150.0), (260.0, 150.0))
        .press((260.0, 150.0))
        .drag_to((300.0, 150.0))
        .chord(CMD, Key::Char('z'));
    assert_eq!(app.rect("p1").x, 200.0);
}

#[test]
fn armed_tool_takes_priority_over_handles_and_alt() {
    let mut app = p1_selected();
    app.key(Key::Char('c')).hold(ALT).press(P1_CORNER);
    assert!(matches!(app.session().gesture, Some(Gesture::Comment(_))));
}

#[test]
fn comment_drag_over_a_page_makes_a_region_in_that_pages_document() {
    let mut app = armed();
    app.tick(86_400_000)
        .drag((200.0, 200.0), (300.0, 250.0))
        .answer_grab(&[1])
        .type_text("here")
        .key(Key::Enter);
    let note = &app.document().annotations()[0];
    let anchor = note.page_anchor.as_ref().unwrap();
    assert_eq!(
        note.anchor,
        AnnotationAnchor::Region(RegionAnchor::Document {
            doc_rect: Rect::new(100.0, 100.0, 100.0, 50.0)
        })
    );
    assert_eq!(
        (
            anchor.page_id.as_str(),
            anchor.page_url.as_deref(),
            note.created_at.as_str(),
            region_on_canvas(app.app(), note)
        ),
        (
            "p1",
            Some("https://example.com/p1"),
            "1970-01-02T00:00:00.000Z",
            Some(Rect::new(200.0, 200.0, 100.0, 50.0))
        )
    );
}

#[test]
fn a_page_region_travels_with_its_page() {
    let mut app = armed();
    app.drag((200.0, 200.0), (300.0, 250.0))
        .answer_grab(&[1])
        .type_text("here")
        .key(Key::Enter)
        .tool(Tool::Select)
        .press((150.0, 150.0))
        .drag_to((650.0, 350.0));
    let note = &app.document().annotations()[0];
    assert_eq!(
        region_on_canvas(app.app(), note),
        Some(Rect::new(700.0, 400.0, 100.0, 50.0))
    );
}

#[test]
fn comment_drag_over_empty_canvas_makes_a_canvas_region() {
    let mut app = armed();
    app.drag((550.0, 500.0), (650.0, 560.0))
        .type_text("here")
        .key(Key::Enter);
    assert_eq!(
        app.document().annotations()[0].anchor,
        AnnotationAnchor::Region(RegionAnchor::Canvas {
            canvas_rect: Rect::new(550.0, 500.0, 100.0, 60.0)
        })
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_comment_is_one_undo_step_and_each_gets_its_own_id() {
    let mut app = armed();
    app.drag((200.0, 200.0), (300.0, 300.0))
        .answer_grab(&[1])
        .type_text("one")
        .key(Key::Enter)
        .drag((550.0, 500.0), (650.0, 560.0))
        .type_text("two")
        .key(Key::Enter);
    let ids: Vec<_> = app
        .document()
        .annotations()
        .iter()
        .map(|note| note.id.clone())
        .collect();
    app.key(Key::Escape).chord(CMD, Key::Char('z'));
    assert_eq!(
        (
            ids.len(),
            ids[0] != ids[1],
            app.document().annotations().len()
        ),
        (2, true, 1)
    );
    app.redo().assert_undo_returns_to_start();
}
