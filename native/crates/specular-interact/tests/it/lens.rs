//! The lens of an item's tab, the eye, and leaving it.
//!
//! Each test names the change that breaks it:
//! - what is seen: making Fill show everything draws the canvas's neighbours
//!   over an item that fills the view, making the eye open by default puts
//!   them around a Device tab, and reading the eye in Fill or dropping
//!   `Others::Hooked` loses what is placed on a page in Device.
//! - filling: returning `None` for a page from `showing::fill_rect` leaves
//!   it at its stored size, dropping `follow_layouts` from `update`
//!   leaves its host there, and dropping the presented size from
//!   `pages::snapshot` resizes the host when a preset is picked.
//! - the camera: dropping the refit from `showing::settle` lets the wheel
//!   pan a Device tab, and entering the page in `show_item` whatever the
//!   lens gives the wheel to the page.
//! - leaving: centring the item in `showing::leave` on anything but the free
//!   part of the viewport, or at any zoom but the canvas's, moves the item as
//!   the camera lands, and saving `session.camera` puts the item view's
//!   camera on disk.
//! - a click outside: clearing the selection for it in `marquee::finish`
//!   leaves a Device tab showing with nothing selected, and dropping the
//!   lens from that rule takes a click on Fill's margin out of the tab.
//! - per tab: keeping one lens in the session moves every tab at once, and
//!   keeping the eye per tab opens it for one only. Dropping the
//!   `Effect::Save` from `showing::enter`, `set_lens` or `close_tab` loses
//!   that tab, lens or close at the next launch.
//! - making: dropping the `refuses` rule makes a shape that is hidden at
//!   once, and leaving `only_page` set in Device hooks a shape made beside
//!   the page to it.
//! - reach: reading `hides` in `reveal::items` leaves a sidebar row doing
//!   nothing while the camera is held on another item.

use glam::Vec2;
use specular_core::Camera;
use specular_core::CssSize;
use specular_doc::{AnchorElement, Entity, ItemId, JsonMap, PageAnchor, Rect};
use specular_interact::{
    Action, Effect, Focus, Hit, Key, Lens, Property, Showing, Tool, hit_test, named_controls, seen,
};
use specular_testkit::{SHIFT, TestApp, note, page, sticky};

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
fn the_lens_and_the_eye_decide_what_is_seen_beside_the_item() {
    let rows = [
        // In Fill the item is the whole view: only what follows it, whatever
        // the eye says.
        (Lens::Fill, false, vec!["on"]),
        (Lens::Fill, true, vec!["on"]),
        // Device starts with the eye shut: the page and what is hooked to it.
        (Lens::Device, false, vec!["on"]),
        (Lens::Device, true, vec!["on", "by", "p2", "n"]),
    ];
    for (lens, eye, want) in rows {
        let mut app = canvas();
        app.act(show("p1")).act(Action::SetLens(lens));
        assert!(!app.app().shows_others(), "the eye starts shut");
        app.act(Action::ShowOthers(eye));
        assert_eq!(others_seen(&app), want, "{lens:?}, eye {eye}");
        assert!(seen(app.app(), app.entity("p1")).is_some());
        // What is not seen takes no press: under the hooked sticky is the
        // page.
        let under = hit_test(app.app(), at(&app, 170.0, 170.0));
        let sticky = matches!(under, Hit::EntityBody { ref entity } if entity.as_str() == "on");
        assert_eq!(sticky, want.contains(&"on"), "{lens:?}: {under:?}");
    }

    // Shutting the eye takes what it hides out of the selection.
    let mut app = canvas();
    app.act(show("p1")).act(Action::SetLens(Lens::Device));
    app.act(Action::ShowOthers(true)).key(Key::Escape);
    app.select(&["by"]).act(Action::ShowOthers(false));
    assert_eq!(app.selected_ids(), [] as [&str; 0]);

    // A page that fills the view is laid out at another width, so what is
    // hooked to it by position alone is seen only where the page is stored,
    // eye shut or not.
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        hooked("placed", false),
    ]);
    app.act(show("p1"));
    assert!(seen(app.app(), app.entity("placed")).is_none());
    app.act(Action::SetLens(Lens::Device));
    assert!(seen(app.app(), app.entity("placed")).is_some());
}

#[test]
fn device_holds_the_camera_and_leaving_centres_the_item_at_the_canvas_zoom() {
    let mut app = canvas();
    let canvas_camera = Camera::new(Vec2::new(-320.0, 75.0), 0.5);
    app.act(Action::SetCamera(canvas_camera));
    app.act(show("p1")).act(Action::SetLens(Lens::Device));
    assert_eq!(app.session().focus, Focus::Page("p1".into()));
    let fitted = app.session().camera;
    // Over the bare canvas beside the page.
    app.key(Key::Escape).pointer_move((900.0, 700.0));
    app.wheel((120.0, 80.0));
    assert_eq!(app.session().camera, fitted, "Device holds the camera");

    // What a save writes is still the canvas's own camera.
    let id = app.app().space().active().id.clone();
    assert_eq!(app.app().canvas_camera(), canvas_camera);
    assert_eq!(
        app.app().canvas_to_save(&id).map(|saved| saved.camera),
        Some(canvas_camera)
    );

    // Leaving keeps the zoom and centres the page, (300, 250) in the middle
    // of the 1000 x 800 window.
    app.act(Action::Show(Showing::Canvas));
    assert_eq!(
        app.session().camera,
        Camera::new(Vec2::new(350.0, 275.0), 0.5)
    );
    assert_eq!(app.app().canvas_camera(), app.session().camera);

    // Closing the tab showing leaves the same way, whatever tabs are left.
    app.act(show("p2")).act(show("p1"));
    app.act(Action::CloseTab(None));
    assert_eq!(app.app().showing(), Showing::Canvas);
    assert_eq!(app.session().camera.zoom, 0.5);
    app.act(show("p1")).act(Action::CloseTab(Some("p2".into())));
    assert_eq!(app.app().showing(), Showing::Item("p1".into()));
}

#[test]
fn a_click_on_the_canvas_around_a_device_tab_goes_back_as_minimize_does() {
    // Bare canvas beside the page in Device; the margin around it in Fill.
    let bare = (900.0, 700.0);
    let mut app = canvas();
    app.act(show("p1")).act(Action::SetLens(Lens::Device));

    // A drag there is still a marquee, and Shift still only extends.
    app.drag(bare, (940.0, 740.0));
    app.hold(SHIFT).click(bare).let_go();
    assert_eq!(app.app().showing(), Showing::Item("p1".into()));

    let mut app = canvas();
    app.act(show("p1")).act(Action::SetLens(Lens::Device));
    app.click(bare);
    let mut minimized = canvas();
    minimized.act(show("p1")).act(Action::SetLens(Lens::Device));
    minimized.act(Action::Show(Showing::Canvas));
    assert_eq!(app.app().showing(), Showing::Canvas);
    assert_eq!(app.selected_ids(), minimized.selected_ids());
    assert_eq!(app.session().camera, minimized.session().camera);
    assert_eq!(app.session().focus, Focus::Canvas);

    let mut app = canvas();
    app.act(show("p1")).click((1.0, 400.0));
    assert_eq!(app.app().showing(), Showing::Item("p1".into()));
}

#[test]
fn the_lens_is_kept_by_each_tab() {
    let mut app = canvas();
    app.with_panels();
    let before = app.document().clone();
    app.take_effects();
    let control = |app: &TestApp, name: &str| {
        (named_controls(app.app()).iter()).any(|id| id.as_str() == name)
    };
    // The Canvas tab has no item to look at, so no control.
    assert_eq!(app.app().lens(), None);
    assert!(!control(&app, "view.lens.device") && !control(&app, "view.minimize"));

    // The open tabs and their lenses are written with the canvas, so a
    // change to them asks for a save and nothing else here does.
    let saves = |app: &mut TestApp| {
        (app.take_effects().iter()).any(|effect| matches!(effect, Effect::Save))
    };

    // A tab starts in Fill, and Minimize is there to go back.
    app.act(show("p1"));
    assert_eq!(app.app().lens(), Some(Lens::Fill));
    assert!(control(&app, "view.minimize"));
    assert!(saves(&mut app), "a tab was opened");
    // The eye is Device's, and one for every tab.
    assert!(!control(&app, "view.others"));
    app.click_control("view.lens.device");
    assert_eq!(app.app().lens(), Some(Lens::Device));
    assert!(saves(&mut app), "its lens changed");
    app.click_control("view.others");
    assert!(app.app().shows_others());
    assert!(!saves(&mut app), "the eye is not saved");

    app.act(show("p2"));
    assert_eq!(app.app().lens(), Some(Lens::Fill), "p2 has its own lens");
    assert!(app.app().shows_others(), "the eye is one for every tab");
    app.take_effects();
    app.click_control("view.minimize");
    assert_eq!(app.app().showing(), Showing::Canvas);

    app.click_control("view.item.p1");
    assert_eq!(app.app().lens(), Some(Lens::Device), "p1 kept its lens");
    assert!(!saves(&mut app), "which tab is showing is not saved");

    app.act(Action::CloseTab(Some("p2".into())));
    assert!(saves(&mut app), "a tab was closed");

    // None of it is a change to the document.
    assert_eq!(*app.document(), before);
    assert!(!app.app().can_undo());
}

#[test]
fn what_is_made_beside_the_item_is_kept_when_it_is_seen_and_not_made_when_it_is_not() {
    let on = (250.0, 200.0);
    let beside = (800.0, 250.0);
    // In the margin of the page that fills the view.
    let margin = (95.0, 95.0);
    // (the lens, the eye, the canvas point a shape is dragged out from,
    // whether it is made, the page it is hooked to)
    let rows = [
        (Lens::Device, true, beside, true, None),
        (Lens::Device, true, on, true, Some("p1")),
        (Lens::Device, false, beside, false, None),
        (Lens::Device, false, on, true, Some("p1")),
        (Lens::Fill, false, margin, false, None),
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
    // Seen in Device with the eye open, but the camera cannot go to it.
    let mut app = canvas();
    app.act(show("p1")).act(Action::SetLens(Lens::Device));
    app.act(Action::ShowOthers(true));
    app.act(reveal("p2"));
    assert_eq!(app.app().showing(), Showing::Canvas);
    assert_eq!(app.selected_ids(), ["p2"]);
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

#[test]
fn a_page_that_fills_the_view_is_a_desktop_browser_whatever_its_device() {
    let mut app = TestApp::from_canvas(
        r#"{"nodes":[{"id":"p1","type":"link","x":100,"y":100,"width":393,"height":852,
            "url":"https://example.com/p1","metadata":{"deviceId":"iphone-14-pro"}}],"edges":[]}"#,
    );
    app.viewport(Vec2::new(1200.0, 800.0));
    // Whether the host was told it is a touch device by each step, and
    // whether the app treats it as one after.
    let told = |app: &mut TestApp| {
        let sent: Vec<bool> = (app.take_effects().into_iter())
            .filter_map(|effect| match effect {
                Effect::SetPageDevice { device, .. } => Some(device.touch),
                _ => None,
            })
            .collect();
        (sent, app.app().page_device(&"p1".into()).touch)
    };
    assert_eq!(told(&mut app), (vec![], true));

    app.act(show("p1"));
    assert_eq!(told(&mut app), (vec![false], false), "Fill");
    app.act(Action::SetLens(Lens::Device));
    assert_eq!(told(&mut app), (vec![true], true), "Device");
    app.act(Action::SetLens(Lens::Fill));
    assert_eq!(told(&mut app), (vec![false], false), "Fill again");
    app.act(Action::Show(Showing::Canvas));
    assert_eq!(told(&mut app), (vec![true], true), "the canvas");
}
