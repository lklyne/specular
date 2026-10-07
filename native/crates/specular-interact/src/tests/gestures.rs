//! Move, resize and the comment-region drag, with undo.

use specular_core::CssSize;
use specular_doc::{AnnotationAnchor, Rect, RegionAnchor};

use super::*;
use crate::{Gesture, region_on_canvas};

#[test]
fn alt_drag_moves_the_page_by_the_pointer_delta() {
    let mut app = app();
    let effects = press_with(&mut app, (200.0, 150.0), ALT);
    drag_to(&mut app, (260.0, 130.0));
    assert_eq!(
        (effects, rect_of(&app, "p1"), selected(&app)),
        (
            Vec::new(),
            Rect::new(160.0, 80.0, 400.0, 300.0),
            Some(id("p1"))
        )
    );
}

#[test]
fn move_is_a_drag_until_release_and_the_page_sees_none_of_it() {
    let mut app = app();
    press_with(&mut app, (200.0, 150.0), ALT);
    let moved = drag_to(&mut app, (210.0, 150.0));
    let during = app.session().gesture.is_some();
    let released = release(&mut app, (210.0, 150.0));
    assert_eq!(
        (during, app.session().gesture.is_none(), moved, released),
        (true, true, Vec::new(), Vec::new())
    );
}

#[test]
fn move_keeps_the_page_size_and_viewport() {
    let mut app = app();
    press_with(&mut app, (200.0, 150.0), ALT);
    drag_to(&mut app, (900.0, 900.0));
    let placement = app.page_placement(&id("p1")).unwrap();
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
    let mut app = app();
    set_zoom(&mut app, 0.5);
    press_with(&mut app, (100.0, 75.0), ALT);
    drag_to(&mut app, (110.0, 75.0));
    assert_eq!(rect_of(&app, "p1").x, 120.0);
}

#[test]
fn a_move_is_one_undo_step_however_many_frames_it_took() {
    let mut app = app();
    press_with(&mut app, (200.0, 150.0), ALT);
    drag_to(&mut app, (220.0, 150.0));
    drag_to(&mut app, (260.0, 130.0));
    release(&mut app, (260.0, 130.0));
    let moved = rect_of(&app, "p1");
    key_with(&mut app, Key::Char('z'), true, CMD);
    let undone = rect_of(&app, "p1");
    let nothing_left = !app.can_undo();
    key_with(&mut app, Key::Char('z'), true, CMD_SHIFT);
    assert_eq!(
        (moved, undone, nothing_left, rect_of(&app, "p1")),
        (
            Rect::new(160.0, 80.0, 400.0, 300.0),
            Rect::new(100.0, 100.0, 400.0, 300.0),
            true,
            Rect::new(160.0, 80.0, 400.0, 300.0)
        )
    );
}

#[test]
fn a_press_and_release_without_movement_is_not_an_undo_step() {
    let mut app = app();
    press_with(&mut app, (200.0, 150.0), ALT);
    release(&mut app, (200.0, 150.0));
    assert!(!app.can_undo());
}

#[test]
fn handle_press_resizes_instead_of_selecting_the_page_under_it() {
    // p2 overlaps p1's bottom-right corner and paints above it.
    let mut app = app_with(vec![
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        page("p2", Rect::new(450.0, 350.0, 400.0, 300.0)),
    ]);
    select(&mut app, "p1");
    let effects = press(&mut app, (500.0, 400.0));
    assert!(matches!(
        app.session().gesture,
        Some(Gesture::Resize { .. })
    ));
    assert_eq!((effects, selected(&app)), (Vec::new(), Some(id("p1"))));
}

#[test]
fn handles_only_exist_on_the_selected_page() {
    let mut app = app();
    press(&mut app, (500.0, 400.0));
    assert_eq!(app.session().gesture, None);
}

#[test]
fn resize_drag_changes_the_rect_but_not_the_viewport() {
    let mut app = app();
    select(&mut app, "p1");
    press(&mut app, (500.0, 400.0));
    let effects = drag_to(&mut app, (600.0, 450.0));
    let placement = app.page_placement(&id("p1")).unwrap();
    assert_eq!(
        (effects, placement.rect, placement.viewport),
        (
            Vec::new(),
            Rect::new(100.0, 100.0, 500.0, 350.0),
            CssSize::new(400, 300)
        )
    );
}

#[test]
fn resize_release_sets_the_viewport_once() {
    let mut app = app();
    select(&mut app, "p1");
    press(&mut app, (500.0, 400.0));
    drag_to(&mut app, (600.0, 450.0));
    drag_to(&mut app, (620.0, 470.0));
    let first = release(&mut app, (620.0, 470.0));
    let second = release(&mut app, (620.0, 470.0));
    assert_eq!(
        (first, second),
        (
            vec![Effect::SetPageViewport {
                page: id("p1"),
                viewport: CssSize::new(520, 370)
            }],
            Vec::new()
        )
    );
}

#[test]
fn resize_from_the_top_left_moves_the_origin() {
    let mut app = app();
    select(&mut app, "p1");
    press(&mut app, (102.0, 98.0));
    drag_to(&mut app, (52.0, 48.0));
    assert_eq!(rect_of(&app, "p1"), Rect::new(50.0, 50.0, 450.0, 350.0));
}

#[test]
fn handle_grab_offset_prevents_a_jump_on_press() {
    let mut app = app();
    select(&mut app, "p1");
    press(&mut app, (503.0, 403.0));
    drag_to(&mut app, (503.0, 403.0));
    assert_eq!(rect_of(&app, "p1"), Rect::new(100.0, 100.0, 400.0, 300.0));
}

#[test]
fn resize_without_a_size_change_does_nothing() {
    let mut app = app();
    select(&mut app, "p1");
    press(&mut app, (500.0, 400.0));
    let effects = release(&mut app, (500.0, 400.0));
    assert_eq!((effects, app.can_undo()), (Vec::new(), false));
}

#[test]
fn undoing_a_resize_lays_the_page_out_at_its_old_viewport() {
    let mut app = app();
    select(&mut app, "p1");
    press(&mut app, (500.0, 400.0));
    drag_to(&mut app, (600.0, 450.0));
    release(&mut app, (600.0, 450.0));
    let effects = key_with(&mut app, Key::Char('z'), true, CMD);
    assert_eq!(
        (effects, rect_of(&app, "p1")),
        (
            vec![Effect::SetPageViewport {
                page: id("p1"),
                viewport: CssSize::new(400, 300)
            }],
            Rect::new(100.0, 100.0, 400.0, 300.0)
        )
    );
}

#[test]
fn escape_snaps_a_move_back_and_leaves_no_undo_step() {
    let mut app = app();
    press_with(&mut app, (200.0, 150.0), ALT);
    drag_to(&mut app, (300.0, 300.0));
    key(&mut app, Key::Escape);
    assert_eq!(
        (
            rect_of(&app, "p1"),
            app.session().gesture.is_some(),
            app.can_undo()
        ),
        (Rect::new(100.0, 100.0, 400.0, 300.0), false, false)
    );
}

#[test]
fn undo_is_ignored_while_a_drag_is_in_flight() {
    let mut app = app();
    press_with(&mut app, (200.0, 150.0), ALT);
    drag_to(&mut app, (260.0, 150.0));
    release(&mut app, (260.0, 150.0));
    press_with(&mut app, (260.0, 150.0), ALT);
    drag_to(&mut app, (300.0, 150.0));
    key_with(&mut app, Key::Char('z'), true, CMD);
    assert_eq!(rect_of(&app, "p1").x, 200.0);
}

#[test]
fn armed_tool_draws_instead_of_selecting_or_forwarding() {
    let mut app = armed();
    let effects = press(&mut app, (200.0, 200.0));
    assert_eq!((effects, selected(&app)), (Vec::new(), None));
}

#[test]
fn armed_tool_takes_priority_over_handles_and_alt() {
    let mut app = app();
    select(&mut app, "p1");
    key(&mut app, Key::Char('c'));
    press_with(&mut app, (500.0, 400.0), ALT);
    assert!(app.session().comment_preview().is_some());
}

#[test]
fn preview_is_the_normalised_drag_rect() {
    let mut app = armed();
    press(&mut app, (300.0, 300.0));
    drag_to(&mut app, (250.0, 340.0));
    assert_eq!(
        app.session().comment_preview(),
        Some(Rect::new(250.0, 300.0, 50.0, 40.0))
    );
}

#[test]
fn comment_drag_over_a_page_makes_a_region_in_that_pages_document() {
    let mut app = armed();
    update(
        &mut app,
        Event::Tick {
            unix_ms: 86_400_000,
        },
    );
    press(&mut app, (200.0, 200.0));
    drag_to(&mut app, (300.0, 250.0));
    release(&mut app, (300.0, 250.0));
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
            region_on_canvas(&app, note)
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
    press(&mut app, (200.0, 200.0));
    release(&mut app, (300.0, 250.0));
    key(&mut app, Key::Escape);
    press_with(&mut app, (150.0, 150.0), ALT);
    drag_to(&mut app, (650.0, 350.0));
    let note = &app.document().annotations()[0];
    assert_eq!(
        region_on_canvas(&app, note),
        Some(Rect::new(700.0, 400.0, 100.0, 50.0))
    );
}

#[test]
fn comment_drag_over_empty_canvas_makes_a_canvas_region() {
    let mut app = armed();
    press(&mut app, (550.0, 500.0));
    drag_to(&mut app, (650.0, 560.0));
    release(&mut app, (650.0, 560.0));
    assert_eq!(
        app.document().annotations()[0].anchor,
        AnnotationAnchor::Region(RegionAnchor::Canvas {
            canvas_rect: Rect::new(550.0, 500.0, 100.0, 60.0)
        })
    );
}

#[test]
fn a_click_with_the_tool_creates_nothing() {
    let mut app = armed();
    press(&mut app, (200.0, 200.0));
    release(&mut app, (201.0, 201.0));
    assert_eq!(app.document().annotations().len(), 0);
}

#[test]
fn the_tool_stays_armed_after_a_drag() {
    let mut app = armed();
    press(&mut app, (200.0, 200.0));
    release(&mut app, (300.0, 300.0));
    assert_eq!(app.session().tool, Tool::Comment);
}

#[test]
fn escape_mid_draw_cancels_without_creating() {
    let mut app = armed();
    press(&mut app, (200.0, 200.0));
    drag_to(&mut app, (300.0, 300.0));
    key(&mut app, Key::Escape);
    release(&mut app, (300.0, 300.0));
    assert_eq!(app.document().annotations().len(), 0);
}

#[test]
fn a_comment_is_one_undo_step_and_each_gets_its_own_id() {
    let mut app = armed();
    press(&mut app, (200.0, 200.0));
    release(&mut app, (300.0, 300.0));
    press(&mut app, (550.0, 500.0));
    release(&mut app, (650.0, 560.0));
    let ids: Vec<_> = app
        .document()
        .annotations()
        .iter()
        .map(|note| note.id.clone())
        .collect();
    key(&mut app, Key::Escape);
    key_with(&mut app, Key::Char('z'), true, CMD);
    assert_eq!(
        (
            ids.len(),
            ids[0] != ids[1],
            app.document().annotations().len()
        ),
        (2, true, 1)
    );
}
