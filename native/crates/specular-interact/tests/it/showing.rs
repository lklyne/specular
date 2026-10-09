//! The tab row and the item view: one page or Document shown alone.
//!
//! Each test names the change that breaks it:
//! - the tabs: sorting `showing::listed` by the stack alone reorders them
//!   after a bring to front.
//! - the item view: dropping the `showing::hides` check from
//!   `scroll_follow::seen` lets a press reach a hidden page.
//! - the round trip: writing the fitted camera without keeping the canvas's
//!   loses it, and saving `session.camera` puts the fitted one on disk.
//! - the fallback: dropping the gone-item check from `showing::settle`
//!   leaves the window showing nothing.
//! - the switch: dropping `showing::leave` from the space's `leave` carries
//!   the view to a canvas whose page has the same id.
//! - a new tab: putting the page in with `live::put` makes no undo step, and
//!   dropping `free_spot::place` lands it on the page beside it.
//! - the address: dropping the Command+L row leaves the key to the page.
//! - next and previous: dropping the `EnteredPage` rows stops at the first
//!   page, which is entered, and dropping the modulo runs off the end.

use glam::Vec2;
use specular_core::{Camera, Modifiers};
use specular_doc::Rect;
use specular_interact::panel::builtin::CHROME_HEIGHT;
use specular_interact::{Action, Effect, Focus, Hit, Key, Showing, hit_test, view_strip};
use specular_testkit::{CMD, TestApp, document, file, note, page, pages, shape};

const CMD_ALT: Modifiers = Modifiers { alt: true, ..CMD };

const VIEWPORT: Vec2 = Vec2::new(1000.0, 800.0);

fn show(id: &str) -> Action {
    Action::Show(Showing::Item(id.into()))
}

/// The tabs as `name label`, the active one marked.
fn tabs(app: &TestApp) -> Vec<String> {
    (view_strip(app.app()).tabs.iter())
        .map(|tab| {
            let mark = if tab.active { " *" } else { "" };
            format!("{} {}{mark}", tab.id, tab.label)
        })
        .collect()
}

fn mixed() -> TestApp {
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        shape("s", Rect::new(100.0, 500.0, 100.0, 100.0)),
        note("n", Rect::new(700.0, 100.0, 300.0, 400.0), "notes/plan.md"),
        file("img", Rect::new(700.0, 600.0, 100.0, 100.0)),
        page("p2", Rect::new(1200.0, 100.0, 390.0, 844.0)),
    ]);
    app.viewport(VIEWPORT);
    app
}

#[test]
fn the_tabs_are_the_canvas_then_each_page_and_document_in_an_order_that_holds() {
    let mut app = mixed();
    let start = [
        "view.canvas Canvas *",
        "view.item.p1 example.com",
        "view.item.n plan",
        "view.item.p2 example.com",
    ];
    assert_eq!(tabs(&app), start);

    // Neither the stack nor a move reorders them, and selecting on the
    // canvas leaves the Canvas tab the active one.
    app.select(&["p1"]).act(Action::BringToFront);
    app.act(Action::Nudge {
        dx: 2000.0,
        dy: 0.0,
    });
    assert_eq!(tabs(&app), start);

    // A new page's tab goes last, and nothing switches to it.
    app.act(Action::Duplicate);
    let copy = app.selected().expect("the copy is selected").to_owned();
    assert_eq!(tabs(&app)[4], format!("view.item.{copy} example.com"));
    assert_eq!(app.app().showing(), Showing::Canvas);
}

#[test]
fn an_item_view_shows_one_item_fitted_and_hides_the_rest_from_presses() {
    let mut app = mixed();
    app.with_panels();
    let over_p1 = app
        .session()
        .camera
        .world_to_screen(Vec2::new(300.0, 250.0));
    app.click_control("view.item.p2");

    assert_eq!(app.app().showing(), Showing::Item("p2".into()));
    assert_eq!(tabs(&app)[3], "view.item.p2 example.com *");
    assert_eq!(app.selected_ids(), ["p2"]);
    // 844 tall into what the chrome leaves, less 64 above and below, and
    // centred there. The page keeps its stored size.
    let camera = app.session().camera;
    let room = VIEWPORT.y - CHROME_HEIGHT - 2.0 * 64.0;
    assert!((camera.zoom - room / 844.0).abs() < 1e-6);
    let centre = camera.world_to_screen(Vec2::new(1395.0, 522.0));
    let middle = Vec2::new(VIEWPORT.x / 2.0, f32::midpoint(VIEWPORT.y, CHROME_HEIGHT));
    assert!(centre.abs_diff_eq(middle, 1e-2), "{centre}");
    assert_eq!(app.rect("p2"), Rect::new(1200.0, 100.0, 390.0, 844.0));

    // p1 now lies under this point and takes no press; p2 does.
    let on_p1 = camera.world_to_screen(Vec2::new(300.0, 250.0));
    assert_eq!(hit_test(app.app(), on_p1), Hit::Empty);
    assert!(matches!(
        hit_test(app.app(), middle),
        Hit::PageContent { ref page, .. } if page.as_str() == "p2"
    ));
    // Nothing that is hidden can be selected, so nothing hidden is deleted.
    app.chord(CMD, Key::Char('a'));
    assert_eq!(app.selected_ids(), ["p2"]);

    // A small page is not magnified.
    app.click_control("view.item.p1");
    assert!((app.session().camera.zoom - 1.0).abs() < 1e-6);

    app.click_control("view.canvas");
    assert_eq!(app.app().showing(), Showing::Canvas);
    assert!(matches!(
        hit_test(app.app(), over_p1),
        Hit::PageContent { ref page, .. } if page.as_str() == "p1"
    ));
}

#[test]
fn the_canvas_camera_survives_an_item_view_and_nothing_is_written() {
    let mut app = mixed();
    let canvas = Camera::new(Vec2::new(-320.0, 75.0), 0.5);
    app.act(Action::SetCamera(canvas));
    let before = app.document().clone();
    app.take_effects();

    app.act(show("n"));
    let fitted = app.session().camera;
    assert_ne!(fitted, canvas);
    // The camera that is saved is still the canvas's.
    assert_eq!(app.app().canvas_camera(), canvas);
    let id = app.app().space().active().id.clone();
    assert_eq!(
        app.app().canvas_to_save(&id).map(|saved| saved.1),
        Some(canvas)
    );

    // The view is held on the item: a pan does not move it, and a resize
    // fits it again.
    app.wheel((120.0, 80.0));
    assert_eq!(app.session().camera, fitted);
    app.viewport((600.0, 500.0));
    assert_ne!(app.session().camera, fitted);

    app.act(show("p1")).act(Action::Show(Showing::Canvas));
    assert_eq!(app.session().camera, canvas);
    assert_eq!(*app.document(), before);
    assert!(!app.app().can_undo());
    let saves = (app.take_effects().iter()).any(|effect| matches!(effect, Effect::Save));
    assert!(!saves, "switching views is not a change to save");
}

#[test]
fn an_item_view_whose_item_goes_falls_back_to_the_canvas() {
    // Deleted while shown.
    let mut app = mixed();
    let canvas = app.session().camera;
    // Escape first: the page shown is entered, and Delete is its own.
    app.act(show("p1")).key(Key::Escape).key(Key::Delete);
    assert_eq!(app.app().showing(), Showing::Canvas);
    assert_eq!(app.session().camera, canvas);
    assert_eq!(tabs(&app).len(), 3);

    // Taken back by an undo while shown.
    let mut app = mixed();
    app.select(&["p1"]).act(Action::Duplicate);
    let copy = app.selected().expect("the copy is selected").to_owned();
    let canvas = app.session().camera;
    app.act(show(&copy));
    assert_eq!(app.app().showing(), Showing::Item(copy.as_str().into()));
    app.undo();
    assert_eq!(app.app().showing(), Showing::Canvas);
    assert_eq!(app.session().camera, canvas);
}

#[test]
fn a_canvas_switch_does_not_carry_an_item_view_across() {
    // Both canvases have a page `p1`.
    let mut app =
        TestApp::with_space([("Home", document(pages(2))), ("Other", document(pages(1)))]);
    app.viewport(VIEWPORT);
    let home = Camera::new(Vec2::new(40.0, 20.0), 0.75);
    app.act(Action::SetCamera(home)).act(show("p1"));
    app.switch_to("Other");
    assert_eq!(app.app().showing(), Showing::Canvas);
    assert_eq!(
        tabs(&app),
        ["view.canvas Canvas *", "view.item.p1 example.com"]
    );
    app.switch_to("Home");
    assert_eq!(app.app().showing(), Showing::Canvas);
    assert_eq!(app.session().camera, home);
}

#[test]
fn a_new_tab_is_a_page_in_a_free_spot_shown_with_the_caret_in_its_address() {
    let mut app = mixed();
    app.with_panels().select(&["p1"]).click_control("view.add");
    let made = app.selected().expect("the new page is selected").to_owned();
    assert_eq!(app.app().showing(), Showing::Item(made.as_str().into()));
    assert_eq!(tabs(&app)[4], format!("view.item.{made} Page *"));
    // It is clear of everything else on the canvas.
    let rect = app.rect(&made);
    let clear = (app.document().entities())
        .filter(|entity| entity.id.as_str() != made)
        .all(|other| {
            let it = other.rect;
            rect.x >= it.x + it.width
                || it.x >= rect.x + rect.width
                || rect.y >= it.y + it.height
                || it.y >= rect.y + rect.height
        });
    assert!(clear, "{rect:?}");

    // The address has the keys, and what is typed there is where it goes.
    assert_eq!(app.field_edit(), Some(""));
    app.take_effects();
    app.type_text("example.org").key(Key::Enter);
    let went = (app.take_effects().iter())
        .any(|effect| matches!(effect, Effect::Navigate { page, .. } if page.as_str() == made));
    assert!(went, "the page is sent to the address");

    // Command+T does the same from inside a page. Making the page is one
    // undo step, and undoing it goes back to the canvas.
    let mut app = mixed();
    app.act(show("p1")).chord(CMD, Key::Char('t'));
    let made = app.selected().expect("the new page is selected").to_owned();
    assert_eq!(app.app().showing(), Showing::Item(made.as_str().into()));
    app.key(Key::Escape).undo();
    assert_eq!(app.app().showing(), Showing::Canvas);
    assert!(app.document().entity(&made.as_str().into()).is_none());
    assert!(!app.app().can_undo(), "one step");
}

#[test]
fn command_l_puts_the_caret_in_the_address_of_the_page_in_the_dock() {
    let mut app = mixed();
    // Selected on the canvas, and entered in its own tab.
    app.select(&["p1"]).chord(CMD, Key::Char('l'));
    assert_eq!(app.field_edit(), Some("https://example.com/p1"));
    app.key(Key::Escape).act(show("p2"));
    assert_eq!(app.session().focus, Focus::Page("p2".into()));
    app.chord(CMD, Key::Char('l'));
    assert_eq!(app.field_edit(), Some("https://example.com/p2"));

    // With no page in the dock there is no address.
    app.key(Key::Escape)
        .act(show("n"))
        .chord(CMD, Key::Char('l'));
    assert_eq!(app.field_edit(), None);
}

#[test]
fn next_and_previous_tab_go_round() {
    let mut app = mixed();
    let shown = |app: &TestApp| match app.app().showing() {
        Showing::Canvas => "canvas".to_owned(),
        Showing::Item(item) => item.as_str().to_owned(),
    };
    // Forward through every tab, pages entered on the way, and round.
    let mut forward = Vec::new();
    for _ in 0..4 {
        app.chord(CMD_ALT, Key::ArrowRight);
        forward.push(shown(&app));
    }
    assert_eq!(forward, ["p1", "n", "p2", "canvas"]);
    // Back from the first tab is the last.
    app.chord(CMD_ALT, Key::ArrowLeft);
    assert_eq!(shown(&app), "p2");
    app.chord(CMD_ALT, Key::ArrowLeft);
    assert_eq!(shown(&app), "n");
}
