//! The tab row and the item view: one page or Document shown alone.
//!
//! Each test names the change that breaks it:
//! - the tabs: sorting `showing::listed` by the stack alone reorders them
//!   after a bring to front.
//! - the item view: dropping the `showing::hides` check from
//!   `scroll_follow::seen` lets a press reach a hidden page. A tab starts in
//!   the Fill lens, which is what these show; `lens.rs` has the others.
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
//! - a press: see the test.

use glam::Vec2;
use specular_core::{Camera, Modifiers};
use specular_doc::Rect;
use specular_interact::panel::builtin::CHROME_HEIGHT;
use specular_interact::{
    Action, Effect, Focus, Hit, Key, Lens, PageNotice, Showing, hit_test, named_controls,
    view_strip,
};
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
fn a_tab_is_opened_for_an_item_and_closed_without_touching_the_canvas() {
    let mut app = mixed();
    app.with_panels();
    let before = app.document().clone();
    let open =
        |app: &TestApp| (named_controls(app.app()).iter()).any(|id| id.as_str() == "view.open");
    // Nothing has a tab until it is opened in one, and only one page or
    // Document selected can be.
    assert_eq!(tabs(&app), ["view.canvas Canvas *"]);
    assert!(!open(&app));
    for (selection, offered) in [
        (&["s"][..], false),
        (&["img"], false),
        (&["p1", "p2"], false),
        (&["n"], true),
        (&["p1"], true),
    ] {
        app.select(selection);
        assert_eq!(open(&app), offered, "{selection:?}");
    }

    app.click_control("view.open");
    assert_eq!(app.app().showing(), Showing::Item("p1".into()));
    assert!(!open(&app), "the item shown has its tab");
    app.act(show("n")).act(show("p2"));
    let all = [
        "view.canvas Canvas",
        "view.item.p1 example.com",
        "view.item.n plan",
        "view.item.p2 example.com *",
    ];
    assert_eq!(tabs(&app), all);

    // The tabs keep the order they were opened in: neither the stack nor
    // showing one again reorders them.
    app.act(Action::Show(Showing::Canvas));
    app.select(&["p2"]).act(Action::SendToBack);
    app.click_control("view.item.n");
    assert_eq!(tabs(&app)[2], "view.item.n plan *");
    assert_eq!(tabs(&app).len(), 4);

    // Closing a tab shows the one that takes its place, then the one
    // before it, then the canvas, which stays.
    let canvas = app.app().canvas_camera();
    let mut shown = Vec::new();
    for _ in 0..4 {
        app.chord(CMD, Key::Char('w'));
        shown.push(tabs(&app).join(", "));
    }
    assert_eq!(
        shown,
        [
            "view.canvas Canvas, view.item.p1 example.com, view.item.p2 example.com *",
            "view.canvas Canvas, view.item.p1 example.com *",
            "view.canvas Canvas *",
            "view.canvas Canvas *",
        ]
    );
    assert_eq!(app.session().camera, canvas);

    // A tab's own close button takes it away whichever tab is showing,
    // and what is shown stays unless it was that one.
    app.act(show("p1")).act(show("n")).act(show("p2"));
    app.control("view.item.p1.close").expect("p1 has a tab");
    assert_eq!(
        tabs(&app)[1..],
        ["view.item.n plan", "view.item.p2 example.com *"]
    );
    app.control("view.item.p2.close").expect("p2 has a tab");
    assert_eq!(tabs(&app)[1..], ["view.item.n plan *"]);
    app.control("view.item.n.close").expect("n has a tab");
    assert_eq!(tabs(&app), ["view.canvas Canvas *"]);

    // A closed tab's item is still on the canvas, and opens again.
    app.undo();
    assert_eq!(*app.document(), before);
    app.act(show("n"));
    assert_eq!(tabs(&app)[1], "view.item.n plan *");
}

#[test]
fn an_item_view_shows_one_item_fitted_and_hides_the_rest_from_presses() {
    let mut app = mixed();
    app.with_panels();
    let over_p1 = app
        .session()
        .camera
        .world_to_screen(Vec2::new(300.0, 250.0));
    app.select(&["p2"]).click_control("view.open");

    assert_eq!(app.app().showing(), Showing::Item("p2".into()));
    assert_eq!(tabs(&app)[1], "view.item.p2 example.com *");
    assert_eq!(app.selected_ids(), ["p2"]);
    // The page fills what the chrome leaves at 100%, four pixels in from
    // each edge. Its stored size is kept.
    let camera = app.session().camera;
    assert!((camera.zoom - 1.0).abs() < 1e-6);
    let corner = camera.world_to_screen(Vec2::new(1200.0, 100.0));
    assert!(
        corner.abs_diff_eq(Vec2::new(4.0, CHROME_HEIGHT + 4.0), 1e-2),
        "{corner}"
    );
    let middle = Vec2::new(VIEWPORT.x / 2.0, f32::midpoint(VIEWPORT.y, CHROME_HEIGHT));
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
    assert_eq!(tabs(&app), ["view.canvas Canvas *"]);

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
    assert_eq!(tabs(&app), ["view.canvas Canvas *"]);
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
    assert_eq!(tabs(&app)[1], format!("view.item.{made} Page *"));
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
    for item in ["p1", "n", "p2"] {
        app.act(show(item)).key(Key::Escape);
    }
    app.act(Action::Show(Showing::Canvas));
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

/// The viewport changes among `effects`, as `(page, width, height)`.
fn viewports(effects: &[Effect]) -> Vec<(String, u32, u32)> {
    (effects.iter())
        .filter_map(|effect| match effect {
            Effect::SetPageViewport { page, viewport } => {
                Some((page.to_string(), viewport.width, viewport.height))
            }
            _ => None,
        })
        .collect()
}

/// Breaks when `pages::laid_out` forgets the prepared page (the host is
/// resized at the click, and the old frame flashes in the tab), when
/// `page_placement` or `view` follows it (the canvas redraws under a held
/// press), or when `prepare` takes anything but a page that is not showing.
#[test]
fn a_press_on_a_tab_lays_its_page_out_and_the_click_shows_it() {
    let prepare = |id: &str| Action::PrepareShow(Some(id.into()));
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        page("p2", Rect::new(700.0, 100.0, 400.0, 300.0)),
        note("n", Rect::new(100.0, 600.0, 300.0, 400.0), "plan.md"),
    ]);
    app.viewport((1000.0, 800.0));
    let before = app.document().clone();
    let drawn = app.scene_snapshot();
    app.take_effects();

    // The press sizes the host for the tab, and nothing drawn follows it.
    app.act(prepare("p1"));
    assert_eq!(
        viewports(&app.take_effects()),
        [("p1".to_owned(), 992, 792)]
    );
    assert_eq!(app.app().prepared_page(), Some(&"p1".into()));
    assert_eq!(app.app().showing(), Showing::Canvas);
    assert_eq!(app.scene_snapshot(), drawn);

    // Let go elsewhere, the host goes back.
    app.act(Action::PrepareShow(None));
    assert_eq!(
        viewports(&app.take_effects()),
        [("p1".to_owned(), 400, 300)]
    );
    assert_eq!(app.app().prepared_page(), None);

    // The click shows the tab with the host already there.
    app.act(prepare("p1")).take_effects();
    app.act(show("p1"));
    assert_eq!(viewports(&app.take_effects()), []);
    assert_eq!(app.app().showing(), Showing::Item("p1".into()));
    assert_eq!(app.app().prepared_page(), None);

    // A press on another page's tab leaves the one showing as it fills,
    // and showing anything else lets the pressed one go.
    app.act(prepare("p2"));
    assert_eq!(
        viewports(&app.take_effects()),
        [("p2".to_owned(), 992, 792)]
    );
    app.act(Action::Show(Showing::Canvas));
    assert_eq!(
        viewports(&app.take_effects()),
        [("p1".to_owned(), 400, 300), ("p2".to_owned(), 400, 300)]
    );

    // A Document's tab, a tab that is not in Fill and the tab showing have
    // nothing to lay out.
    app.act(prepare("n"));
    app.act(show("p2"))
        .act(Action::SetLens(Lens::Device))
        .act(show("p1"));
    app.take_effects();
    app.act(prepare("p2")).act(prepare("p1"));
    assert_eq!(viewports(&app.take_effects()), []);
    assert_eq!(app.app().prepared_page(), None);

    // Nothing of it was written, and there is nothing to undo.
    assert_eq!(*app.document(), before);
    assert!(!app.app().can_undo());
}

#[test]
fn a_page_tab_has_the_pages_icon_until_another_document_loads() {
    let mut app = mixed();
    let favicon = |app: &TestApp, id: &str| {
        let strip = view_strip(app.app());
        let tab = strip.tabs.iter().find(|tab| tab.id.to_string() == id);
        tab.and_then(|tab| tab.favicon.clone())
    };
    let png: std::sync::Arc<[u8]> = vec![0x89, b'P', b'N', b'G'].into();
    app.act(show("p1")).act(show("p2")).act(show("n"));
    app.page_reports("p1", PageNotice::Url("https://example.com/a".into()))
        .page_reports("p1", PageNotice::Favicon(Some(png.clone())));
    assert_eq!(favicon(&app, "view.item.p1"), Some(png));
    assert_eq!(favicon(&app, "view.item.p2"), None);
    assert_eq!(favicon(&app, "view.item.n"), None);

    // The same document at another hash keeps it; another document's own
    // has not arrived yet.
    app.page_reports("p1", PageNotice::Url("https://example.com/a#top".into()));
    assert!(favicon(&app, "view.item.p1").is_some());
    app.page_reports("p1", PageNotice::Url("https://example.com/b".into()));
    assert_eq!(favicon(&app, "view.item.p1"), None);
}
