//! Living in an item view through the Fill lens, which a tab starts in:
//! the page shown is entered, and what is made there lands on it. The other
//! lenses and the eye are in `lens.rs`.
//!
//! Each test names the change that breaks it:
//! - entered: dropping `set_focus` from `showing::show` leaves the wheel
//!   panning a camera that is held still, and dropping the seen check from
//!   `hit::page_at` gives the wheel to a hidden page lying over the spot.
//! - one click: dropping `showing::holds` from `select::press_page` makes
//!   the click only select the page.
//! - creation: dropping `refused` from `pointer::tool_takes_press` makes
//!   something the view hides at once.
//! - hooked: dropping the `alone` fallback from `anchor::page_anchor_for`
//!   frees a stroke whose middle is off the page, and dropping `creating`
//!   from `showing::hides` hides it while it is drawn.
//! - hidden pages: dropping `scrolls.offers` from `page_anchor_for` hooks a
//!   shape to a page that is not shown.
//! - paste: dropping the anchor from the text paste, or the gate from
//!   `asset::insert_selected`, pastes something hidden.
//! - reading: dropping `showing::presented_rect` from `placed_rect` wraps
//!   the text at the stored width, from `showing::fitted` centres the stored
//!   rect, and from `handle_target` lets a drag at the corner write the
//!   column's size. Dropping `showing::holds` from the edit arm of
//!   `select::press` leaves a click selecting. Returning the column from
//!   `presented_rect` in every lens leaves Device with no card to resize.

use glam::Vec2;
use specular_core::InputEvent;
use specular_doc::{Entity, Kind, PageAnchor, Rect};
use specular_interact::{Action, Effect, Focus, Key, Lens, Showing, Tool, seen, shown_rect};
use specular_testkit::{CMD, TestApp, note, page, sticky};

fn show(id: &str) -> Action {
    Action::Show(Showing::Item(id.into()))
}

/// `p1` with a sticky `s` hooked to it and a page `over` lying across its
/// right half in front, a second page `p2` and a Document `n`.
fn canvas() -> TestApp {
    let hooked = Entity {
        anchor: Some(PageAnchor::new("p1".into())),
        ..sticky("s", Rect::new(120.0, 120.0, 100.0, 100.0), "hi")
    };
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        hooked,
        page("over", Rect::new(300.0, 100.0, 400.0, 300.0)),
        page("p2", Rect::new(1200.0, 100.0, 400.0, 300.0)),
        note("n", Rect::new(100.0, 600.0, 300.0, 400.0), "plan.md"),
    ]);
    app.viewport((1000.0, 800.0));
    app
}

/// The canvas point `(x, y)` on screen, as the camera has it now.
fn at(app: &TestApp, x: f32, y: f32) -> Vec2 {
    app.session().camera.world_to_screen(Vec2::new(x, y))
}

fn entered(app: &TestApp) -> Option<&str> {
    match &app.session().focus {
        Focus::Page(page) => Some(page.as_str()),
        Focus::Canvas => None,
    }
}

fn anchor_of<'a>(app: &'a TestApp, id: &str) -> Option<&'a str> {
    (app.entity(id).anchor.as_ref()).map(|anchor| anchor.page_id.as_str())
}

#[test]
fn a_shown_page_is_entered_and_the_wheel_scrolls_it() {
    let mut app = canvas();
    app.act(show("p1"));
    assert_eq!(entered(&app), Some("p1"));

    // Over the half of p1 that `over` covers on the canvas.
    let spot = at(&app, 450.0, 250.0);
    app.pointer_move(spot);
    app.take_effects();
    app.wheel((0.0, -40.0));
    let scrolled = app.take_effects().iter().any(|effect| {
        matches!(effect, Effect::ForwardInput { page, event: InputEvent::Wheel(_) }
            if page.as_str() == "p1")
    });
    assert!(scrolled, "the wheel goes to the page shown");

    // Back on the canvas the page is only selected.
    app.act(Action::Show(Showing::Canvas));
    assert_eq!(entered(&app), None);
    assert_eq!(app.selected_ids(), ["p1"]);
}

#[test]
fn escape_leaves_a_shown_page_and_one_click_enters_it_again() {
    let mut app = canvas();
    app.act(show("p1")).key(Key::Escape);
    assert_eq!(entered(&app), None);
    // The canvas has the keys.
    app.chord(CMD, Key::Char('b'));
    assert!(
        app.app().covered_left() > 0.0,
        "Command+B shows the sidebar"
    );
    app.chord(CMD, Key::Char('b'));

    // Whatever is selected, one click on the page's body goes into it.
    app.select(&["s"]);
    let body = at(&app, 300.0, 350.0);
    app.click(body);
    assert_eq!(entered(&app), Some("p1"));
    assert_eq!(app.selected_ids(), ["p1"]);
}

#[test]
fn a_creation_press_makes_nothing_an_item_view_would_hide() {
    // (the item shown, the tool, the canvas point pressed)
    let off = (800.0, 250.0);
    let on = (300.0, 350.0);
    let rows = [
        ("p1", Tool::AddSticky, off),
        ("p1", Tool::AddText, off),
        ("p1", Tool::AddShape, off),
        ("p1", Tool::Draw, off),
        ("p1", Tool::Comment, off),
        // A page or a Document is never hooked to the page shown.
        ("p1", Tool::AddPage, on),
        ("p1", Tool::AddDocument, on),
        // Nothing is hooked to a Document.
        ("n", Tool::AddSticky, (150.0, 650.0)),
    ];
    for (item, tool, (x, y)) in rows {
        let mut app = canvas();
        app.act(show(item)).key(Key::Escape);
        let before = app.document().clone();
        app.take_effects();
        let spot = at(&app, x, y);
        app.tool(tool).drag(spot, spot + Vec2::new(60.0, 60.0));
        assert_eq!(*app.document(), before, "{tool:?} in {item}");
        assert!(app.app().text_edit().is_none(), "{tool:?} in {item}");
        assert!(app.app().comment_draft().is_none(), "{tool:?} in {item}");
        let asked =
            (app.take_effects().iter()).any(|effect| matches!(effect, Effect::CreateNote { .. }));
        assert!(!asked, "{tool:?} in {item} asks for no file");
    }
}

#[test]
fn what_is_made_on_a_shown_page_is_hooked_to_it_and_seen() {
    let mut app = canvas();
    app.act(show("p1"));
    let spot = at(&app, 300.0, 300.0);
    app.tool(Tool::AddSticky).click(spot).type_text("note");
    let made = app.selected().expect("the sticky is selected").to_owned();
    app.key(Key::Escape);
    assert_eq!(anchor_of(&app, &made), Some("p1"));
    assert!(seen(app.app(), app.entity(&made)).is_some());

    // A stroke that starts on the page and runs out of it has its middle
    // off the page. It is seen while it is drawn, and stays.
    let (from, to) = (at(&app, 480.0, 380.0), at(&app, 900.0, 380.0));
    app.tool(Tool::Draw).press(from).drag_to(to);
    let drawing = app.app().creating().expect("a stroke in flight").clone();
    let live = app
        .document()
        .entity(&drawing)
        .expect("it is in the document");
    assert!(seen(app.app(), live).is_some(), "seen while it is drawn");
    app.release();
    let stroke = (app.document().entities())
        .find(|entity| matches!(entity.kind, Kind::Drawing(_)))
        .expect("the stroke");
    assert_eq!(
        stroke.anchor.as_ref().map(|it| it.page_id.as_str()),
        Some("p1")
    );
    assert!(seen(app.app(), stroke).is_some());
}

#[test]
fn nothing_is_hooked_to_a_page_an_item_view_hides() {
    let mut app = canvas();
    app.act(show("p1")).key(Key::Escape);
    // Dragged out from p1 until its middle, (600, 250), is where `over`
    // lies on the canvas and p1 does not.
    let (from, to) = (at(&app, 480.0, 120.0), at(&app, 720.0, 380.0));
    app.tool(Tool::AddShape).drag(from, to);
    let made = app.selected().expect("the shape is selected").to_owned();
    assert_eq!(anchor_of(&app, &made), Some("p1"));

    // On the canvas the same drag hooks it to the page in front there.
    let mut app = canvas();
    app.tool(Tool::AddShape)
        .drag((480.0, 120.0), (720.0, 380.0));
    let made = app.selected().expect("the shape is selected").to_owned();
    assert_eq!(anchor_of(&app, &made), Some("over"));
}

#[test]
fn a_paste_in_an_item_view_lands_on_the_page_shown_or_not_at_all() {
    let mut app = canvas();
    app.act(show("p1")).key(Key::Escape);
    let spot = at(&app, 300.0, 350.0);
    app.pointer_move(spot).paste("a thought");
    let made = app.selected().expect("the sticky is selected").to_owned();
    assert_eq!(anchor_of(&app, &made), Some("p1"));
    assert!(seen(app.app(), app.entity(&made)).is_some());

    // A page would be hidden as soon as it was pasted, and so would anything
    // in a Document's view.
    let before = app.document().clone();
    app.paste("https://example.org");
    assert_eq!(*app.document(), before);
    app.act(show("n")).paste("another thought");
    assert_eq!(*app.document(), before);
}

#[test]
fn a_document_fills_its_tab_as_a_reading_column_and_its_rect_is_not_written() {
    let stored = Rect::new(100.0, 600.0, 300.0, 400.0);
    let mut app = canvas();
    app.note_text("plan.md", "# Plan\n\nOne line.");
    app.act(show("n"));

    // 720 wide and as tall as the fit leaves room for, in the middle of the
    // viewport at 100%. The text wraps inside its padding.
    let column = shown_rect(app.app(), app.entity("n")).expect("the Document is seen");
    assert_eq!((column.width, column.height), (720.0, 800.0 - 128.0));
    assert!((app.session().camera.zoom - 1.0).abs() < 1e-6);
    let middle = at(&app, column.x as f32 + 360.0, column.y as f32 + 336.0);
    assert!(
        middle.abs_diff_eq(Vec2::new(500.0, 400.0), 1e-2),
        "{middle}"
    );
    let wrap = |app: &TestApp| {
        let frame = app.app().text_frame(&"n".into()).expect("a text frame");
        frame.spec.wrap_width
    };
    assert_eq!(wrap(&app), Some(720.0 - 24.0));
    // A window too narrow for the measure gives it what there is.
    app.viewport((600.0, 500.0));
    assert_eq!(wrap(&app), Some(600.0 - 128.0 - 24.0));
    app.viewport((1000.0, 800.0));

    // One click edits it, with the caret where it landed: under the text
    // here, so at its end.
    app.click(middle).type_text("x").key(Key::Escape);
    assert_eq!(app.document().note("plan.md"), Some("# Plan\n\nOne line.x"));

    // The column has no handles, so a drag at its corner resizes nothing.
    let corner = at(
        &app,
        (column.x + column.width) as f32,
        (column.y + column.height) as f32,
    );
    app.drag(corner, corner + Vec2::new(-80.0, -80.0));
    assert_eq!(app.rect("n"), stored);

    // Device shows the card the canvas has, fitted, with its handles.
    app.act(Action::SetLens(Lens::Device));
    assert_eq!(shown_rect(app.app(), app.entity("n")), Some(stored));
    assert_eq!(wrap(&app), Some(300.0 - 24.0));
    let centre = at(&app, 250.0, 800.0);
    assert!(
        centre.abs_diff_eq(Vec2::new(500.0, 400.0), 1e-2),
        "{centre}"
    );
    assert_eq!(app.app().handle_target().map(|it| it.1), Some(stored));

    app.act(Action::Show(Showing::Canvas));
    assert_eq!(shown_rect(app.app(), app.entity("n")), Some(stored));
    assert_eq!(app.rect("n"), stored);
}
