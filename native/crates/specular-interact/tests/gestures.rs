//! Move, resize and the comment tool's region drag, with undo, scripted
//! through `specular-testkit`.

use specular_core::CssSize;
use specular_doc::{AnnotationAnchor, EntityId, Rect, RegionAnchor};
use specular_interact::{Cursor, Effect, Gesture, Key, Tool, region_on_canvas};
use specular_testkit::{ALT, CMD, CMD_SHIFT, TestApp, assert_doc_snapshot, page};

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
fn move_is_a_drag_until_release_and_the_page_sees_none_of_it() {
    let mut app = TestApp::with_pages(2);
    app.press((200.0, 150.0)).drag_to((210.0, 150.0));
    let during = app.session().gesture.is_some();
    app.release();
    assert_eq!(
        (during, app.session().gesture.is_none(), app.take_effects()),
        (true, true, vec![Effect::Save])
    );
}

#[test]
fn move_keeps_the_page_size_and_viewport() {
    let mut app = TestApp::with_pages(2);
    app.press((200.0, 150.0)).drag_to((900.0, 900.0));
    let placement = app.app().page_placement(&EntityId::from("p1")).unwrap();
    assert_eq!(
        (
            placement.rect.width,
            placement.rect.height,
            placement.viewport
        ),
        (400.0, 300.0, CssSize::new(400, 300))
    );
}

#[test]
fn move_follows_the_camera_zoom() {
    let mut app = TestApp::with_pages(2);
    app.zoom(0.5).press((100.0, 75.0)).drag_to((110.0, 75.0));
    assert_eq!(app.rect("p1").x, 120.0);
}

#[test]
fn a_move_is_one_undo_step_however_many_frames_it_took() {
    let mut app = TestApp::with_pages(2);
    app.press((200.0, 150.0))
        .drag_to((220.0, 150.0))
        .drag_to((260.0, 130.0))
        .release();
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"p1","type":"link","x":160,"y":80,"width":400,"height":300,"url":"https://example.com/p1"}
      {"id":"p2","type":"link","x":700,"y":100,"width":400,"height":300,"url":"https://example.com/p2"}
    edges:
    specular: {"entityOrder":["p1","p2"]}
    "#);

    app.chord(CMD, Key::Char('z'));
    assert_eq!((app.rect("p1"), app.app().can_undo()), (P1, false));
    app.chord(CMD_SHIFT, Key::Char('z'));
    assert_eq!(app.rect("p1"), Rect::new(160.0, 80.0, 400.0, 300.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_press_and_release_without_movement_is_not_an_undo_step() {
    let mut app = TestApp::with_pages(2);
    app.click((200.0, 150.0));
    assert!(!app.app().can_undo());
}

#[test]
fn handle_press_resizes_instead_of_selecting_the_page_under_it() {
    // p2 overlaps p1's bottom-right corner and paints above it.
    let mut app = TestApp::with_entities([
        page("p1", P1),
        page("p2", Rect::new(450.0, 350.0, 400.0, 300.0)),
    ]);
    app.select(&["p1"]).press(P1_CORNER);
    assert!(matches!(app.session().gesture, Some(Gesture::Resize(_))));
    assert_eq!(app.selected(), Some("p1"));
}

#[test]
fn handles_only_exist_on_the_selected_page() {
    let mut app = TestApp::with_pages(2);
    app.press(P1_CORNER);
    assert!(matches!(app.session().gesture, Some(Gesture::Move(_))));
}

#[test]
fn resize_drag_changes_the_rect_but_not_the_viewport() {
    let mut app = p1_selected();
    app.press(P1_CORNER).take_effects();
    app.drag_to((600.0, 460.0));
    let placement = app.app().page_placement(&EntityId::from("p1")).unwrap();
    assert_eq!(
        (app.take_effects(), placement.rect, placement.viewport),
        (
            Vec::new(),
            Rect::new(100.0, 100.0, 500.0, 360.0),
            CssSize::new(400, 300)
        )
    );
}

#[test]
fn resize_release_sets_the_viewport_once() {
    let mut app = p1_selected();
    app.press(P1_CORNER).take_effects();
    app.drag_to((600.0, 460.0)).drag_to((620.0, 480.0));
    let first = app.release().take_effects();
    let second = app.release().take_effects();
    assert_eq!(
        (first, second),
        (
            vec![
                Effect::SetPageViewport {
                    page: EntityId::from("p1"),
                    viewport: CssSize::new(520, 380)
                },
                Effect::Save
            ],
            Vec::new()
        )
    );
}

#[test]
fn resize_from_the_top_left_moves_the_origin() {
    let mut app = p1_selected();
    app.press((102.0, 98.0)).drag_to((42.0, 38.0));
    assert_eq!(app.rect("p1"), Rect::new(40.0, 40.0, 460.0, 360.0));
}

#[test]
fn handle_grab_offset_prevents_a_jump_on_press() {
    let mut app = p1_selected();
    app.press((503.0, 403.0)).drag_to((503.0, 403.0));
    assert_eq!(app.rect("p1"), P1);
}

#[test]
fn resize_without_a_size_change_does_nothing() {
    let mut app = p1_selected();
    app.pointer_move(P1_CORNER).take_effects();
    app.click(P1_CORNER);
    assert_eq!(
        (app.take_effects(), app.app().can_undo()),
        (Vec::new(), false)
    );
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
fn escape_snaps_a_move_back_and_leaves_no_undo_step() {
    let mut app = TestApp::with_pages(2);
    app.press((200.0, 150.0))
        .drag_to((300.0, 300.0))
        .key(Key::Escape);
    assert_eq!(
        (
            app.rect("p1"),
            app.session().gesture.is_some(),
            app.app().can_undo()
        ),
        (P1, false, false)
    );
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
fn armed_tool_draws_instead_of_selecting_or_forwarding() {
    let mut app = armed();
    app.press((200.0, 200.0));
    assert_eq!((app.take_effects(), app.selected()), (Vec::new(), None));
}

#[test]
fn armed_tool_takes_priority_over_handles_and_alt() {
    let mut app = p1_selected();
    app.key(Key::Char('c')).hold(ALT).press(P1_CORNER);
    assert!(matches!(app.session().gesture, Some(Gesture::Comment(_))));
}

#[test]
fn preview_is_the_normalised_drag_rect() {
    let mut app = armed();
    app.press((300.0, 300.0)).drag_to((250.0, 340.0));
    assert_eq!(
        app.session().comment_preview(),
        Some(Rect::new(250.0, 300.0, 50.0, 40.0))
    );
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
fn a_click_with_the_tool_puts_nothing_in_the_document_until_its_text_is_committed() {
    let mut app = armed();
    app.drag((600.0, 500.0), (601.0, 501.0));
    assert_eq!(
        (
            app.document().annotations().len(),
            app.app().comment_draft().is_some()
        ),
        (0, true)
    );
}

#[test]
fn the_tool_stays_armed_after_a_drag() {
    let mut app = armed();
    app.drag((200.0, 200.0), (300.0, 300.0));
    assert_eq!(app.session().tool, Tool::Comment);
}

#[test]
fn escape_mid_draw_cancels_without_creating() {
    let mut app = armed();
    app.press((200.0, 200.0))
        .drag_to((300.0, 300.0))
        .key(Key::Escape)
        .release();
    assert_eq!(
        (
            app.document().annotations().len(),
            app.app().comment_draft(),
            app.effects()
                .iter()
                .any(|effect| matches!(effect, Effect::QueryRegionGrab { .. }))
        ),
        (0, None, false)
    );
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
