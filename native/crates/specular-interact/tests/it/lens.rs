//! The lens of an item's tab and the eye.
//!
//! Each test names the change that breaks it:
//! - the eye: dropping the `Others::None` arm from `gates::hides` leaves a
//!   sticky hooked to the page drawn with the eye shut. Making Fill show
//!   `Others::All` draws the canvas's neighbours over an item that fills the
//!   view, and making Device show `Others::Hooked` loses the stickies beside
//!   a page.
//! - the camera: dropping the `held` check from `showing::settle` refits a
//!   panned Canvas lens, and entering the page in `show_item` whatever the
//!   lens gives the wheel to the page.
//! - the round trip: dropping `keep_camera` from `settle` brings a tab back
//!   fitted, and saving `session.camera` puts the tab's camera on disk.
//! - per tab: keeping one lens in the session moves every tab at once, and
//!   keeping the eye in `Tabs` opens it again on the next tab.
//! - making: dropping `Others::None` from `gates::refuses` makes a shape
//!   that is hidden at once, and leaving `only_page` set in Device hooks a
//!   shape made beside the page to it.
//! - reach: reading `hides` in `reveal::items` leaves a sidebar row doing
//!   nothing while the camera is held on another item.

use glam::Vec2;
use specular_core::Camera;
use specular_doc::{Entity, ItemId, PageAnchor, Rect};
use specular_interact::{
    Action, Effect, Focus, Hit, Key, Lens, Showing, Tool, hit_test, named_controls, seen,
};
use specular_testkit::{TestApp, document, note, page, pages, sticky};

fn show(id: &str) -> Action {
    Action::Show(Showing::Item(id.into()))
}

/// `p1` with a sticky `on` hooked to it and a sticky `by` beside it, a
/// second page `p2` and a Document `n`.
fn canvas() -> TestApp {
    let hooked = Entity {
        anchor: Some(PageAnchor::new("p1".into())),
        ..sticky("on", Rect::new(120.0, 120.0, 100.0, 100.0), "on it")
    };
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        hooked,
        sticky("by", Rect::new(540.0, 120.0, 100.0, 100.0), "beside it"),
        page("p2", Rect::new(1200.0, 100.0, 400.0, 300.0)),
        note("n", Rect::new(100.0, 600.0, 300.0, 400.0), "plan.md"),
    ]);
    app.viewport((1000.0, 800.0));
    app
}

fn at(app: &TestApp, x: f32, y: f32) -> Vec2 {
    app.session().camera.world_to_screen(Vec2::new(x, y))
}

/// Which of the items beside `p1` are seen.
fn others_seen(app: &TestApp) -> Vec<&'static str> {
    (["on", "by", "p2", "n"].into_iter())
        .filter(|id| seen(app.app(), app.entity(id)).is_some())
        .collect()
}

#[test]
fn the_eye_and_the_lens_decide_what_is_seen_beside_the_item() {
    let all = vec!["on", "by", "p2", "n"];
    let rows = [
        // In Fill the item is the whole view: only what is on it.
        (Lens::Fill, true, vec!["on"]),
        (Lens::Device, true, all.clone()),
        (Lens::Canvas, true, all),
        // Shut, the eye leaves the item alone, whatever the lens.
        (Lens::Fill, false, vec![]),
        (Lens::Device, false, vec![]),
        (Lens::Canvas, false, vec![]),
    ];
    for (lens, eye, want) in rows {
        let mut app = canvas();
        app.act(show("p1"))
            .act(Action::SetLens(lens))
            .act(Action::ShowOthers(eye));
        assert_eq!(others_seen(&app), want, "{lens:?}, eye {eye}");
        assert!(seen(app.app(), app.entity("p1")).is_some());
        // What is not seen takes no press: under the hooked sticky is the
        // page.
        let under = hit_test(app.app(), at(&app, 170.0, 170.0));
        let sticky = matches!(under, Hit::EntityBody { ref entity } if entity.as_str() == "on");
        assert_eq!(
            sticky,
            want.contains(&"on"),
            "{lens:?}, eye {eye}: {under:?}"
        );
    }

    // Shutting the eye takes what it hides out of the selection, and on the
    // Canvas tab it does nothing.
    let mut app = canvas();
    app.act(show("p1")).act(Action::SetLens(Lens::Canvas));
    app.select(&["by"]).act(Action::ShowOthers(false));
    assert_eq!(app.selected_ids(), [] as [&str; 0]);
    app.act(Action::Show(Showing::Canvas));
    assert_eq!(others_seen(&app), ["on", "by", "p2", "n"]);
}

#[test]
fn the_canvas_lens_frees_the_camera_and_device_holds_it() {
    let mut app = canvas();
    app.act(show("p1")).act(Action::SetLens(Lens::Device));
    assert_eq!(app.session().focus, Focus::Page("p1".into()));
    let fitted = app.session().camera;
    // Over the bare canvas beside the page.
    app.key(Key::Escape).pointer_move((900.0, 700.0));
    app.wheel((120.0, 80.0));
    assert_eq!(app.session().camera, fitted, "Device holds the camera");

    // The Canvas lens starts where Device was, with the page selected and
    // not entered, as on the canvas.
    app.act(show("p1")).act(Action::SetLens(Lens::Canvas));
    assert_eq!(app.session().camera, fitted);
    assert_eq!(app.session().focus, Focus::Canvas);
    assert_eq!(app.selected_ids(), ["p1"]);
    app.take_effects();
    app.pointer_move(at(&app, 300.0, 250.0))
        .wheel((120.0, 80.0));
    assert_eq!(
        app.session().camera.pan,
        fitted.pan + Vec2::new(120.0, 80.0)
    );
    let forwarded =
        (app.take_effects().iter()).any(|effect| matches!(effect, Effect::ForwardInput { .. }));
    assert!(!forwarded, "the wheel pans, it does not scroll the page");
    // A resize leaves a free camera where it is.
    let panned = app.session().camera;
    app.viewport((600.0, 500.0));
    assert_eq!(app.session().camera, panned);

    // Back in Device it is fitted again.
    app.viewport((1000.0, 800.0))
        .act(Action::SetLens(Lens::Device));
    assert_eq!(app.session().camera, fitted);
}

#[test]
fn a_tab_keeps_its_canvas_lens_camera_and_the_canvas_keeps_the_one_saved() {
    let mut app =
        TestApp::with_space([("Home", document(pages(2))), ("Other", document(pages(1)))]);
    app.viewport((1000.0, 800.0));
    let canvas = Camera::new(Vec2::new(-320.0, 75.0), 0.5);
    app.act(Action::SetCamera(canvas));

    app.act(show("p1")).act(Action::SetLens(Lens::Canvas));
    app.pointer_move((500.0, 700.0)).wheel((-60.0, 40.0));
    let left = app.session().camera;
    assert_ne!(left, canvas);
    // What a save writes is still the canvas's own camera.
    let id = app.app().space().active().id.clone();
    assert_eq!(app.app().canvas_camera(), canvas);
    assert_eq!(
        app.app().canvas_to_save(&id).map(|saved| saved.1),
        Some(canvas)
    );

    // Another tab, the Canvas tab and another canvas, and back each time.
    app.act(show("p2"));
    assert_ne!(app.session().camera, left);
    app.act(show("p1"));
    assert_eq!(app.session().camera, left);
    app.act(Action::Show(Showing::Canvas));
    assert_eq!(app.session().camera, canvas);
    app.switch_to("Other").switch_to("Home").act(show("p1"));
    assert_eq!(app.app().lens(), Some(Lens::Canvas));
    assert_eq!(app.session().camera, left);
    assert_eq!(app.app().canvas_camera(), canvas);
}

#[test]
fn the_lens_is_kept_by_each_tab_and_the_eye_by_all_of_them() {
    let mut app = canvas();
    app.with_panels();
    let before = app.document().clone();
    app.take_effects();
    let control = |app: &TestApp, name: &str| {
        (named_controls(app.app()).iter()).any(|id| id.as_str() == name)
    };
    // The Canvas tab has no item to look at, so no control.
    assert_eq!(app.app().lens(), None);
    assert!(!control(&app, "view.lens.device") && !control(&app, "view.others"));

    // A tab starts in Fill with the eye open.
    app.click_control("view.item.p1");
    assert_eq!(app.app().lens(), Some(Lens::Fill));
    assert!(app.app().shows_others());
    app.click_control("view.lens.device");
    assert_eq!(app.app().lens(), Some(Lens::Device));

    app.click_control("view.item.p2");
    assert_eq!(app.app().lens(), Some(Lens::Fill), "p2 has its own lens");
    app.click_control("view.others");
    assert!(!app.app().shows_others());

    app.click_control("view.item.p1");
    assert_eq!(app.app().lens(), Some(Lens::Device), "p1 kept its lens");
    assert!(!app.app().shows_others(), "the eye is one for every tab");
    app.click_control("view.others");
    assert!(app.app().shows_others());

    // None of it is a change to the document.
    assert_eq!(*app.document(), before);
    assert!(!app.app().can_undo());
    let saves = (app.take_effects().iter()).any(|effect| matches!(effect, Effect::Save));
    assert!(!saves, "the lens and the eye are not saved");
}

#[test]
fn what_is_made_beside_the_item_is_kept_when_it_is_seen_and_not_made_when_it_is_not() {
    let on = (250.0, 200.0);
    let beside = (800.0, 250.0);
    // (the lens, the eye, the canvas point a shape is dragged out from,
    // whether it is made, the page it is hooked to)
    let rows = [
        (Lens::Device, true, beside, true, None),
        (Lens::Device, true, on, true, Some("p1")),
        (Lens::Canvas, true, beside, true, None),
        (Lens::Device, false, on, false, None),
        (Lens::Canvas, false, beside, false, None),
        (Lens::Fill, false, on, false, None),
    ];
    for (lens, eye, (x, y), made, hook) in rows {
        let mut app = canvas();
        app.act(show("p1"))
            .act(Action::SetLens(lens))
            .act(Action::ShowOthers(eye))
            .key(Key::Escape);
        let before = app.document().clone();
        let spot = at(&app, x, y);
        app.tool(Tool::AddShape)
            .drag(spot, spot + Vec2::new(60.0, 60.0));
        let case = format!("{lens:?}, eye {eye}, at {x},{y}");
        if !made {
            assert_eq!(*app.document(), before, "{case}");
            // Nor does a paste land anywhere.
            app.pointer_move(spot).paste("a thought");
            assert_eq!(*app.document(), before, "{case}: a paste");
            continue;
        }
        let id = app.selected().expect("the shape is selected").to_owned();
        let shape = app.entity(&id);
        assert!(seen(app.app(), shape).is_some(), "{case}");
        let hooked = shape.anchor.as_ref().map(|it| it.page_id.as_str());
        assert_eq!(hooked, hook, "{case}");
    }
}

#[test]
fn a_reveal_leaves_a_view_whose_camera_is_held_on_another_item() {
    let reveal = |id: &str| Action::Reveal {
        select: vec![ItemId::Entity(id.into())],
        focus: ItemId::Entity(id.into()),
    };
    // Seen in Device, but the camera cannot go to it.
    let mut app = canvas();
    app.act(show("p1")).act(Action::SetLens(Lens::Device));
    app.act(reveal("p2"));
    assert_eq!(app.app().showing(), Showing::Canvas);
    assert_eq!(app.selected_ids(), ["p2"]);

    // In the Canvas lens the camera goes to it and the tab stays.
    let mut app = canvas();
    app.act(show("p1")).act(Action::SetLens(Lens::Canvas));
    let before = app.session().camera;
    app.act(reveal("p2"));
    assert_eq!(app.app().showing(), Showing::Item("p1".into()));
    assert_ne!(app.session().camera, before);
}
