//! How often the built-in panels are laid out, and that the layout kept
//! between reads is never older than the app.
//!
//! A layout build measures text and, with the sidebar shown, walks every
//! item of the canvas, so one pointer move must not do it several times.
//! Before the cache a move built the layout five times over empty canvas and
//! three over a control, and the draw after it once more.

use specular_doc::{Document, Rect};
use specular_interact::panel::builtin::{Pointing, layout, layout_builds, layout_uncached};
use specular_interact::{Action, PageNotice, Tool};
use specular_testkit::{TestApp, document, page, sticky};

/// A free spot on the canvas, clear of the toolbar, the popup and the
/// sidebar.
const EMPTY: (f32, f32) = (1000.0, 800.0);

fn home() -> Document {
    let mut items = vec![page("p1", Rect::new(400.0, 200.0, 400.0, 300.0))];
    for i in 0..60 {
        let at = Rect::new(900.0, 150.0 + f64::from(i) * 4.0, 120.0, 60.0);
        items.push(sticky(&format!("s{i}"), at, "note"));
    }
    document(items)
}

fn app() -> TestApp {
    let notes = document([sticky("n1", Rect::new(100.0, 100.0, 120.0, 60.0), "other")]);
    let mut app = TestApp::with_space([("Home", home()), ("Notes", notes)]);
    app.with_panels();
    app
}

/// What drawing right after an update does.
fn draw(app: &TestApp) -> String {
    app.panel_scene_snapshot()
}

/// How many layouts `step` builds.
fn builds(app: &mut TestApp, step: impl FnOnce(&mut TestApp)) -> u64 {
    let before = layout_builds();
    step(app);
    layout_builds() - before
}

#[track_caller]
fn fresh(app: &TestApp, what: &str) {
    assert_eq!(
        *layout(app.app()),
        layout_uncached(app.app()),
        "the kept layout is stale after {what}"
    );
}

#[test]
fn the_first_read_builds_and_the_draw_after_a_move_reuses_it() {
    let mut app = app();
    assert_eq!(
        builds(&mut app, |app| {
            app.pointer_move(EMPTY);
        }),
        1
    );
    assert_eq!(
        builds(&mut app, |app| {
            draw(app);
        }),
        0
    );
}

#[test]
fn a_move_that_changes_nothing_builds_nothing() {
    for (what, selected, sidebar) in [
        ("empty canvas", None, false),
        ("a selection popup shown", Some("s0"), false),
        ("a sidebar row", None, true),
    ] {
        let mut app = app();
        if let Some(id) = selected {
            app.select(&[id]);
        }
        app.show_sidebar(sidebar);
        draw(&app);
        let at = if sidebar {
            let centre = app.control_rect("sidebar.notes.s59").centre();
            (centre.x, centre.y)
        } else {
            EMPTY
        };
        assert_eq!(
            builds(&mut app, |app| {
                app.pointer_move(at);
            }),
            0,
            "{what}"
        );
        assert_eq!(
            builds(&mut app, |app| {
                draw(app);
            }),
            0,
            "{what}"
        );
    }
}

#[test]
fn a_move_onto_a_popup_control_marks_it_without_a_build() {
    let mut app = app();
    app.select(&["s0"]);
    draw(&app);
    let at = app.control_rect("text.color").centre();
    assert_eq!(
        builds(&mut app, |app| {
            app.pointer_move(at);
        }),
        0
    );
    assert_eq!(
        builds(&mut app, |app| {
            draw(app);
        }),
        0
    );
    let layout = layout(app.app());
    let node = layout.node(&"text.color".to_owned().into());
    assert_eq!(node.map(|node| node.state.pointing), Some(Pointing::Hover));
    fresh(&app, "a hover");
}

#[test]
fn an_update_that_changes_things_builds_once_and_the_draw_after_it_reuses_that() {
    let mut app = app();
    app.show_sidebar(true);
    draw(&app);
    let at = app.control_rect("sidebar.notes.s58").centre();
    assert_eq!(
        builds(&mut app, |app| {
            app.pointer_move(at);
        }),
        0
    );
    assert_eq!(
        builds(&mut app, |app| {
            app.press(at);
        }),
        1
    );
    assert_eq!(
        builds(&mut app, |app| {
            app.release();
        }),
        1
    );
    assert_eq!(app.selected(), Some("s58"));
    assert_eq!(
        builds(&mut app, |app| {
            draw(app);
        }),
        0
    );
}

#[test]
fn a_canvas_drag_builds_once_a_frame() {
    let mut app = app();
    app.select(&["s0"]);
    draw(&app);
    app.press((950.0, 160.0));
    let frame = builds(&mut app, |app| {
        app.drag_to((1000.0, 200.0));
        draw(app);
    });
    assert_eq!(frame, 1);
}

#[test]
fn the_layout_is_never_older_than_the_app() {
    let mut app = app();
    app.select(&["p1"]);
    app.pointer_move(EMPTY);
    fresh(&app, "the start");

    app.select(&["s0"]);
    fresh(&app, "a selection");
    app.tool(Tool::AddShape);
    fresh(&app, "a tool change");
    app.act(Action::SetTool(Tool::Select));
    let popup = layout(app.app()).popup.clone().map(|panel| panel.rect);
    app.wheel((80.0, 60.0));
    assert_ne!(
        layout(app.app()).popup.clone().map(|panel| panel.rect),
        popup
    );
    fresh(&app, "a pan with a popup shown");
    app.show_sidebar(true);
    fresh(&app, "showing the sidebar");
    app.pointer_move(app.control_rect("sidebar.head.notes").centre());
    app.wheel((0.0, -120.0));
    assert!(app.sidebar_scroll() > 0.0, "the sidebar scrolled");
    fresh(&app, "a sidebar scroll");
    app.click_control("sidebar.head.canvases");
    fresh(&app, "folding a section");
    app.select(&["p1"]);
    app.click_control("page.size");
    fresh(&app, "opening a dropdown");
    app.act(Action::Cancel);
    app.click_control("page.url");
    app.type_text("ab");
    fresh(&app, "typing in a field");
    app.key(specular_interact::Key::Escape);
    fresh(&app, "leaving a field");
    app.drag((950.0, 160.0), (1000.0, 200.0));
    fresh(&app, "a drag");
    app.undo();
    fresh(&app, "undo");
    app.page_reports("p1", PageNotice::Title("A new title".to_owned()));
    fresh(&app, "a page title");
    app.page_reports(
        "p1",
        PageNotice::Loading {
            loading: true,
            can_go_back: true,
            can_go_forward: false,
        },
    );
    fresh(&app, "a page loading");
    app.viewport((1200.0, 800.0));
    fresh(&app, "a viewport resize");
    app.switch_to("Notes");
    fresh(&app, "a canvas switch");
    app.switch_to("Home");
    fresh(&app, "switching back");
    app.right_click((950.0, 160.0));
    fresh(&app, "a context menu");
}

#[test]
fn the_sidebars_rows_follow_the_page_the_canvases_and_the_document() {
    let mut app = TestApp::with_space([(
        "Home",
        document([page("p1", Rect::new(400.0, 200.0, 400.0, 300.0))]),
    )]);
    app.with_panels().show_sidebar(true);
    draw(&app);
    fresh(&app, "the start");
    app.page_reports("p1", PageNotice::Title("A long new title".to_owned()));
    fresh(&app, "a page title");
    app.page_reports("p1", PageNotice::Url("https://example.org/next".to_owned()));
    fresh(&app, "a page address");
    app.click_control("sidebar.add");
    fresh(&app, "a new canvas");
    let added = layout(app.app())
        .controls()
        .map(|control| control.as_str().to_owned())
        .find(|name| name.starts_with("sidebar.canvas.") && !name.ends_with("tab_1"))
        .expect("the new canvas has a row");
    let row = app.control_rect(&added).centre();
    app.double_click(row);
    app.enter_in_field(&format!("{added}.name"), "Roadmap");
    assert!(app.panel_snapshot().contains("Roadmap"));
    fresh(&app, "a rename");
    app.tool(Tool::AddSticky).click(EMPTY);
    fresh(&app, "a placed sticky");
    app.undo();
    fresh(&app, "undoing it");
}

#[test]
fn hover_and_press_on_every_control_match_a_fresh_layout() {
    let mut app = app();
    app.show_sidebar(true).select(&["p1"]);
    app.click_control("page.size");
    draw(&app);
    let names: Vec<String> = layout(app.app())
        .controls()
        .map(|control| control.as_str().to_owned())
        .collect();
    assert!(names.len() > 20, "{names:?}");
    for name in names {
        let at = app.control_rect(&name).centre();
        app.pointer_move(at);
        fresh(&app, &format!("hovering {name}"));
        app.press(at);
        fresh(&app, &format!("pressing {name}"));
        app.release();
        app.click(EMPTY);
        app.select(&["p1"]);
        app.click_control("page.size");
    }
}

#[test]
fn the_app_stays_send() {
    fn send<T: Send>() {}
    send::<specular_interact::App>();
}
