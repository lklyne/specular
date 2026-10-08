//! The sidebar in the built-in renderer: what a click on a row does to the
//! selection and the camera, the folds, the scroll, and the width it takes
//! from the canvas.

use glam::Vec2;
use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationId, Entity, EntityId, PageAnchor, Rect,
};
use specular_interact::{Action, ControlId, SIDEBAR_WIDTH, Tool};
use specular_testkit::{
    CMD, SHIFT, TestApp, comment, document, group, inside, page, sticky, with_comment,
};

const BOX: Rect = Rect::new(0.0, 0.0, 200.0, 100.0);
/// What the toolbar and the sidebar leave of a 1600x1000 viewport: its
/// middle is where a revealed item is centred.
const FREE_CENTRE: Vec2 = Vec2::new(928.0, 522.0);

fn at(x: f64, y: f64) -> Rect {
    Rect::new(x, y, 200.0, 100.0)
}

/// A canvas with a sticky in view, one far away and one under the sidebar,
/// and the sidebar shown.
fn app() -> TestApp {
    let mut app = TestApp::with_entities([
        sticky("near", at(600.0, 300.0), "near"),
        sticky("far", at(3000.0, 2000.0), "far"),
        sticky("under", at(40.0, 500.0), "under"),
    ]);
    app.with_panels().show_sidebar(true);
    app
}

fn shown(app: &TestApp, id: &str) -> bool {
    app.panel_layout()
        .controls()
        .any(|control| control.as_str() == id)
}

#[test]
fn a_row_selects_its_item_and_centres_it_in_the_part_the_sidebar_leaves_free() {
    let mut app = app();
    app.click_control("sidebar.notes.far");
    assert_eq!(app.selected(), Some("far"));
    let centre = app
        .session()
        .camera
        .world_to_screen(Vec2::new(3100.0, 2050.0));
    assert_eq!(centre, FREE_CENTRE);
    assert_eq!(
        app.session().camera.zoom,
        1.0,
        "the zoom is the camera's own"
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn an_item_already_in_the_free_part_leaves_the_camera_alone() {
    let mut app = app();
    let before = app.session().camera;
    app.click_control("sidebar.notes.near");
    assert_eq!(app.selected(), Some("near"));
    assert_eq!(app.session().camera, before);
}

#[test]
fn shift_selects_the_run_between_two_rows_and_command_toggles_one() {
    let mut app = TestApp::with_entities([
        sticky("s1", at(600.0, 100.0), "1"),
        sticky("s2", at(600.0, 300.0), "2"),
        sticky("s3", at(600.0, 500.0), "3"),
        sticky("s4", at(600.0, 700.0), "4"),
    ]);
    app.with_panels().show_sidebar(true);
    // Front of the stack first: s4, s3, s2, s1.
    app.click_control("sidebar.notes.s4");
    app.hold(SHIFT).click_control("sidebar.notes.s2").let_go();
    assert_eq!(app.selected_ids(), ["s4", "s3", "s2"]);
    app.hold(CMD).click_control("sidebar.notes.s3").let_go();
    assert_eq!(app.selected_ids(), ["s4", "s2"]);
    app.hold(CMD).click_control("sidebar.notes.s1").let_go();
    assert_eq!(app.selected_ids(), ["s4", "s2", "s1"]);
}

#[test]
fn a_comment_row_focuses_the_comment_and_pans_to_it() {
    let url = "https://example.com/p1";
    let far = Entity {
        rect: Rect::new(3000.0, 2000.0, 400.0, 300.0),
        ..page("p1", BOX)
    };
    let anchor = AnnotationAnchor::Page {
        page_id: EntityId::from("p1"),
        offset_x: 10.0,
        offset_y: 10.0,
    };
    let annotation = Annotation {
        page_anchor: Some(PageAnchor {
            page_url: Some(url.to_owned()),
            ..PageAnchor::new(EntityId::from("p1"))
        }),
        created_at: "2026-01-01T00:00:00.000Z".to_owned(),
        ..comment("c1", anchor, "tighten this")
    };
    let mut app = TestApp::from_document(with_comment(document([far]), annotation));
    app.with_panels().show_sidebar(true);
    app.click_control("sidebar.pages.comment.c1");
    assert_eq!(
        app.app().focused_comment().map(AnnotationId::as_str),
        Some("c1")
    );
    let centre = app
        .session()
        .camera
        .world_to_screen(Vec2::new(3200.0, 2150.0));
    assert_eq!(centre, FREE_CENTRE);
}

#[test]
fn a_group_opens_and_closes_from_its_chevron() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(0.0, 0.0, 600.0, 400.0)),
        inside("g", sticky("m", at(100.0, 100.0), "member")),
    ]);
    app.with_panels().show_sidebar(true);
    assert!(!shown(&app, "sidebar.notes.m"), "a group starts closed");
    app.click_control("sidebar.notes.g.toggle");
    assert!(shown(&app, "sidebar.notes.m"));
    assert!(app.selected().is_none(), "the chevron only opens");
    // The member is one level in: its glyph is 14 px further right.
    let (group_row, member) = (
        app.control_rect("sidebar.notes.g"),
        app.control_rect("sidebar.notes.m"),
    );
    assert_eq!(group_row.width, member.width);
    app.click_control("sidebar.notes.g.toggle");
    assert!(!shown(&app, "sidebar.notes.m"));
}

fn long() -> TestApp {
    let entities =
        (0..60_u32).map(|i| sticky(&format!("s{i}"), at(600.0, 20.0 * f64::from(i)), "x"));
    let mut app = TestApp::with_entities(entities);
    app.with_panels().show_sidebar(true);
    app
}

#[test]
fn a_long_list_scrolls_under_the_wheel_and_never_moves_the_canvas() {
    let mut app = long();
    assert!(
        !shown(&app, "sidebar.notes.s0"),
        "the foot of the list is not laid out"
    );
    let camera = app.session().camera;
    app.pointer_move(Vec2::new(100.0, 400.0))
        .wheel((0.0, -300.0));
    assert_eq!(app.sidebar_scroll(), 300.0);
    assert_eq!(app.session().camera, camera);
    app.wheel((0.0, 100.0));
    assert_eq!(app.sidebar_scroll(), 200.0, "the other way scrolls back");
    app.wheel((0.0, 5000.0));
    assert_eq!(app.sidebar_scroll(), 0.0, "and stops at the top");
    app.wheel((0.0, -50000.0));
    let end = app.sidebar_scroll();
    assert!(end > 0.0);
    app.wheel((0.0, -50.0));
    assert_eq!(app.sidebar_scroll(), end, "and at the end");
}

#[test]
fn a_wheel_off_the_sidebar_still_reaches_the_canvas() {
    let mut app = long();
    let camera = app.session().camera;
    app.pointer_move(Vec2::new(900.0, 600.0))
        .wheel((0.0, -120.0));
    assert_ne!(app.session().camera, camera);
    assert_eq!(app.sidebar_scroll(), 0.0);
}

#[test]
fn the_toolbar_button_shows_and_hides_the_sidebar() {
    let mut app = TestApp::with_entities([sticky("under", at(40.0, 500.0), "under")]);
    app.with_panels();
    assert!(!shown(&app, "sidebar.head.canvases"), "hidden: no panel");
    assert_eq!(app.app().covered_left(), 0.0);
    // Hidden, the press is the canvas's.
    app.click(Vec2::new(100.0, 540.0));
    assert_eq!(app.selected(), Some("under"));
    app.click(Vec2::new(900.0, 800.0));
    app.click_control("sidebar.toggle");
    assert!(shown(&app, "sidebar.head.canvases"));
    assert_eq!(app.app().covered_left(), SIDEBAR_WIDTH);
    assert!(app.selected().is_none());
    // Shown, the press is the sidebar's and never reaches what is under it.
    app.click(Vec2::new(100.0, 540.0));
    assert!(app.selected().is_none());
    app.click_control("sidebar.toggle");
    assert!(!shown(&app, "sidebar.head.canvases"));
}

#[test]
fn hover_selection_and_dimming_are_marked_on_the_rows() {
    use specular_interact::panel::builtin::Pointing;
    let url = "https://example.com/p1";
    let anchored = Entity {
        anchor: Some(PageAnchor {
            page_url: Some("https://example.com/elsewhere".to_owned()),
            ..PageAnchor::new(EntityId::from("p1"))
        }),
        ..sticky("s", BOX, "hooked")
    };
    let mut app = TestApp::with_entities([page("p1", BOX), anchored]);
    let _ = url;
    app.with_panels().show_sidebar(true);
    app.page_reports("p1", specular_interact::PageNotice::Url(url.to_owned()));
    let node = |app: &TestApp, id: &str| {
        app.panel_layout()
            .node(&ControlId::from(id.to_owned()))
            .cloned()
            .expect("a row")
    };
    assert!(node(&app, "sidebar.pages.s").state.dimmed);
    assert!(!node(&app, "sidebar.pages.p1").state.dimmed);
    app.hover_control("sidebar.pages.p1");
    assert_eq!(
        node(&app, "sidebar.pages.p1").state.pointing,
        Pointing::Hover
    );
    app.click_control("sidebar.pages.p1");
    assert!(node(&app, "sidebar.pages.p1").state.on);
}

#[test]
fn zoom_to_fit_centres_what_the_sidebar_leaves_free() {
    let mut app = app();
    app.act(Action::ZoomToFit);
    // The union of the three stickies runs from (40, 300) to (3200, 2100).
    let world_centre = Vec2::new(1620.0, 1200.0);
    let shown_at = app.session().camera.world_to_screen(world_centre);
    assert!((shown_at - FREE_CENTRE).length() < 1.0, "{shown_at}");
    let covered_zoom = app.session().camera.zoom;

    app.show_sidebar(false).act(Action::ZoomToFit);
    let shown_at = app.session().camera.world_to_screen(world_centre);
    assert!(
        (shown_at - Vec2::new(800.0, 522.0)).length() < 1.0,
        "{shown_at}"
    );
    assert!(
        app.session().camera.zoom > covered_zoom,
        "more room, a larger fit"
    );
}

#[test]
fn a_popup_keeps_clear_of_the_sidebar_and_a_tool_popup_centres_beside_it() {
    let mut app = TestApp::with_entities([sticky("t", Rect::new(270.0, 300.0, 100.0, 50.0), "t")]);
    app.with_panels().select(&["t"]);
    let hidden = app.panel_layout().popup.expect("popup").rect;
    assert!(
        hidden.x < 264.0,
        "centred on the sticky it reaches into the strip"
    );
    app.show_sidebar(true);
    let popup = app.panel_layout().popup.expect("popup").rect;
    assert!(popup.x >= SIDEBAR_WIDTH + 8.0, "{popup:?}");

    app.select(&[]).tool(Tool::AddSticky);
    let popup = app.panel_layout().popup.expect("the tool's popup").rect;
    assert!((popup.centre().x - FREE_CENTRE.x).abs() <= 0.5, "{popup:?}");
}
