//! The lens of an item's tab and the eye.
//!
//! Each test names the change that breaks it:
//! - the eye: dropping the `Others::None` arm from `gates::hides` leaves a
//!   sticky hooked to the page drawn with the eye shut. Making Fill show
//!   `Others::All` draws the canvas's neighbours over an item that fills the
//!   view, and making Device show `Others::Following` loses the stickies beside
//!   a page.
//! - filling: returning `None` for a page from `showing::fill_rect` leaves
//!   it at its stored size, dropping `follow_layouts` from `update`
//!   leaves its host there, and dropping the presented size from
//!   `pages::snapshot` resizes the host when a preset is picked.
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
use specular_core::CssSize;
use specular_doc::{AnchorElement, Entity, ItemId, JsonMap, PageAnchor, Rect};
use specular_interact::{
    Action, Effect, Focus, Hit, Key, Lens, Property, Showing, Tool, hit_test, named_controls, seen,
};
use specular_testkit::{TestApp, document, note, page, pages, sticky};

fn show(id: &str) -> Action {
    Action::Show(Showing::Item(id.into()))
}

/// A sticky hooked to `p1`, following an element of it or only placed
/// over it.
fn hooked(id: &str, follows: bool) -> Entity {
    let element = follows.then(|| AnchorElement {
        selector: "main > h1".to_owned(),
        doc_x: 20.0,
        doc_y: 20.0,
        viewport_positioned: None,
        extra: JsonMap::new(),
    });
    Entity {
        anchor: Some(PageAnchor {
            element,
            ..PageAnchor::new("p1".into())
        }),
        ..sticky(id, Rect::new(120.0, 120.0, 100.0, 100.0), "on it")
    }
}

/// `p1` with a sticky `on` that follows an element of it and a sticky `by`
/// beside it, a second page `p2` and a Document `n`.
fn canvas() -> TestApp {
    let hooked = hooked("on", true);
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
        // In Fill the item is the whole view: only what follows it.
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

    // A page that fills the view is laid out at another width, so what is
    // hooked to it by position alone is seen only where the page is stored.
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        hooked("placed", false),
    ]);
    app.act(show("p1"));
    assert!(seen(app.app(), app.entity("placed")).is_none());
    app.act(Action::SetLens(Lens::Device));
    assert!(seen(app.app(), app.entity("placed")).is_some());

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

#[test]
fn a_page_in_fill_is_laid_out_at_the_free_area_and_goes_back_when_left() {
    let mut app = canvas();
    let before = app.document().clone();
    app.take_effects();

    // Shown, the page fills the window at 100% and its host follows.
    app.act(show("p1"));
    assert_eq!(
        viewports(&app.take_effects()),
        [("p1".to_owned(), 992, 792)]
    );
    let placed = app.app().page_placement(&"p1".into()).expect("a page");
    assert_eq!(placed.rect, Rect::new(100.0, 100.0, 992.0, 792.0));
    assert_eq!(placed.viewport, CssSize::new(992, 792));
    assert_eq!(
        app.session().camera,
        Camera::new(Vec2::new(-96.0, -96.0), 1.0)
    );
    assert_eq!(app.session().focus, Focus::Page("p1".into()));

    // The window and the sidebar refit it.
    app.viewport((1200.0, 700.0));
    assert_eq!(
        viewports(&app.take_effects()),
        [("p1".to_owned(), 1192, 692)]
    );
    app.show_sidebar(true);
    let free = 1200 - app.app().covered_left() as u32 - 8;
    assert_eq!(
        viewports(&app.take_effects()),
        [("p1".to_owned(), free, 692)]
    );
    app.show_sidebar(false);
    app.take_effects();

    // A preset picked there is stored, and the host stays as it fills.
    app.act(Action::SetProperty(Property::ViewportPreset(0)));
    assert_ne!(app.rect("p1"), Rect::new(100.0, 100.0, 400.0, 300.0));
    assert_eq!(viewports(&app.take_effects()), []);
    assert_eq!(app.app().lens(), Some(Lens::Fill));
    app.undo();
    app.take_effects();

    // Another lens, another tab and the canvas each put the host back.
    app.act(Action::SetLens(Lens::Device));
    assert_eq!(
        viewports(&app.take_effects()),
        [("p1".to_owned(), 400, 300)]
    );
    app.act(Action::SetLens(Lens::Fill));
    app.take_effects();
    app.act(show("p2"));
    assert_eq!(
        viewports(&app.take_effects()),
        [("p1".to_owned(), 400, 300), ("p2".to_owned(), 1192, 692)]
    );
    app.act(Action::Show(Showing::Canvas));
    assert_eq!(
        viewports(&app.take_effects()),
        [("p2".to_owned(), 400, 300)]
    );

    // Nothing of it was written.
    assert_eq!(*app.document(), before);
}
