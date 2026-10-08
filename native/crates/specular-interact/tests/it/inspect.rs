//! The inspect tool: the node under the pointer is outlined with a popover,
//! a click picks it, and the pick is the chat's turn target until it is
//! cleared.

use glam::Vec2;
use specular_doc::{EntityId, Rect};
use specular_interact::{Action, Effect, Event, Key, PageNotice, PillKind, Tool};
use specular_testkit::{TestApp, page, shape};

/// Over the second row of `p1`'s grid: the page sits at (100, 100), so this
/// is (100, 50) in its CSS pixels.
const CELL_0_1: (f32, f32) = (200.0, 150.0);
/// One cell to the right of it.
const CELL_1_1: (f32, f32) = (280.0, 150.0);

fn armed() -> TestApp {
    let mut app = TestApp::with_pages(2);
    app.viewport((1600.0, 1000.0))
        .tool(Tool::Inspect)
        .take_effects();
    app
}

fn asked(effects: &[Effect]) -> Vec<(Vec2, bool)> {
    (effects.iter())
        .filter_map(|effect| match effect {
            Effect::InspectAt { point, pick, .. } => Some((*point, *pick)),
            _ => None,
        })
        .collect()
}

fn remainder(app: &TestApp) -> Option<String> {
    app.inspect_model().map(|model| model.popover.remainder)
}

fn near(got: Vec2, want: (f32, f32)) {
    assert!(
        (got - Vec2::from(want)).abs().max_element() < 0.01,
        "{got:?} is not {want:?}"
    );
}

#[test]
fn hovering_page_content_outlines_the_node_and_shows_its_popover() {
    let mut app = armed();
    app.pointer_move(CELL_0_1);
    assert_eq!(asked(app.effects()), [(Vec2::new(100.0, 50.0), false)]);
    assert_eq!(app.inspect_model(), None, "nothing shows before the answer");
    app.answer_inspect();
    let model = app.inspect_model().expect("the node is shown");
    // The cell's box, in the page's pixels, on the canvas at zoom 1.
    assert_eq!(
        (model.outline.min, model.outline.size),
        (Vec2::new(100.0, 148.0), Vec2::new(160.0, 48.0))
    );
    let popover = model.popover;
    assert_eq!(popover.tag, "div");
    assert_eq!(popover.remainder, "#cell-0-1.cell");
    assert_eq!(popover.size, "160 \u{d7} 48");
    let font = popover.font.expect("the page reported a font");
    assert_eq!(
        (font.family.as_str(), font.detail.as_str()),
        ("Inter", "14px \u{b7} 400")
    );
    let swatches: Vec<_> = (popover.swatches.iter())
        .map(|s| (s.label.as_str(), s.value.as_str()))
        .collect();
    assert_eq!(
        swatches,
        [("text", "rgb(17, 24, 39)"), ("bg", "rgb(255, 255, 255)")]
    );
    // Above the outline, one gap over it, lined up with its left edge.
    near(popover.rect.size, (324.7, 70.0));
    near(popover.rect.min, (100.0, 72.0));

    // A bare node: no tag, an empty id, a transparent background and no font.
    app.pointer_move(CELL_1_1);
    let node = {
        let mut node = specular_core::synthetic::synthetic_inspected_at(
            specular_core::CssSize::new(400, 300),
            Vec2::new(180.0, 50.0),
        )
        .expect("a cell");
        node.tag_name.clear();
        node.id_attribute = Some(String::new());
        node.classes = ["a", "b", "c", "d"].map(String::from).to_vec();
        node.styles
            .retain(|(name, _)| name != "font-family" && name != "background");
        node.styles
            .push(("background".to_owned(), "rgba(0, 0, 0, 0)".to_owned()));
        node
    };
    app.send(Event::Page {
        page: EntityId::from("p1"),
        notice: PageNotice::Inspected {
            point: Vec2::new(180.0, 50.0),
            pick: false,
            node: Some(Box::new(node)),
        },
    });
    let popover = app.inspect_model().expect("the node is shown").popover;
    assert_eq!(
        (
            popover.tag.as_str(),
            popover.remainder.as_str(),
            popover.font,
            popover.swatches.len()
        ),
        ("element", ".a.b.c", None, 1),
        "a fallback tag, no empty id, three classes, no font, no clear background"
    );
}

#[test]
fn the_page_is_asked_once_a_pixel_and_a_late_answer_is_ignored() {
    let mut app = armed();
    app.pointer_move(CELL_0_1).answer_inspect().take_effects();
    // The same CSS pixel again, and a sub-pixel move within it.
    app.pointer_move(CELL_0_1);
    app.pointer_move((200.6, 150.4));
    assert_eq!(asked(&app.take_effects()), []);
    // A late answer for a point the pointer has left is dropped.
    app.pointer_move(CELL_1_1);
    let late = Event::Page {
        page: EntityId::from("p1"),
        notice: PageNotice::Inspected {
            point: Vec2::new(100.0, 50.0),
            pick: false,
            node: None,
        },
    };
    app.send(late);
    assert_eq!(
        remainder(&app).as_deref(),
        Some("#cell-0-1.cell"),
        "the node before the question stays until the answer"
    );
    app.answer_inspect();
    assert_eq!(remainder(&app).as_deref(), Some("#cell-1-1.cell"));
    // Over the canvas the hover goes, and coming back asks again.
    app.pointer_move((50.0, 50.0));
    assert_eq!(remainder(&app), None);
    app.take_effects();
    app.pointer_move(CELL_1_1);
    assert_eq!(asked(&app.take_effects()).len(), 1);
}

#[test]
fn the_entered_page_hears_nothing_while_the_tool_is_in_hand() {
    let mut app = TestApp::with_pages(2);
    app.double_click(CELL_0_1)
        .tool(Tool::Inspect)
        .take_effects();
    app.pointer_move(CELL_1_1).click(CELL_0_1);
    let effects = app.take_effects();
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::ForwardInput { .. })),
        "{effects:?}"
    );
}

#[test]
fn a_click_picks_the_node_and_it_becomes_the_pill_and_the_turn_target() {
    let mut app = armed();
    app.with_chat_panel().click(CELL_0_1);
    assert_eq!(asked(app.effects()), [(Vec2::new(100.0, 50.0), true)]);
    app.answer_inspect();
    let pill = app.chat().composer.pill;
    assert_eq!(
        (pill.kind, pill.label.as_str()),
        (PillKind::Dom, "div \"Cell 0,1\"")
    );
    assert_eq!(app.app().inspected().map(|t| t.page.as_str()), Some("p1"));
    app.send_chat("make it blue");
    let prompt = app
        .take_effects()
        .into_iter()
        .find_map(|effect| match effect {
            Effect::RunAgent(request) => Some(request.prompt),
            _ => None,
        })
        .expect("Send runs the agent");
    let line = "The user has selected DOM node \"div \"Cell 0,1\"\" on page p1 and likely wants to focus on that.";
    assert!(prompt.contains(line), "{line}\nis missing from\n{prompt}");
}

#[test]
fn the_toolbar_button_toggles_the_tool_and_a_press_off_page_content_selects() {
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        shape("s1", Rect::new(700.0, 100.0, 100.0, 100.0)),
    ]);
    app.with_panels().click_control("tool.inspect");
    assert_eq!(app.session().tool, Tool::Inspect);
    app.click_control("tool.inspect");
    assert_eq!(app.session().tool, Tool::Select);
    app.key(Key::Char('i')).click((750.0, 150.0));
    assert_eq!(app.selected_ids(), ["s1"]);
    assert_eq!(
        app.session().tool,
        Tool::Inspect,
        "the tool rests, like Select"
    );
    assert_eq!(asked(&app.take_effects()), []);
}

/// A pick on `p1`'s second row and the hover on the cell beside it.
fn picked_and_hovering() -> TestApp {
    let mut app = armed();
    app.click(CELL_0_1).answer_inspect();
    app.pointer_move(CELL_1_1).answer_inspect().take_effects();
    assert_eq!(remainder(&app).as_deref(), Some("#cell-1-1.cell"));
    app
}

#[test]
fn a_pick_stays_until_its_page_goes_or_navigates_or_escape_drops_it() {
    type Step = fn(&mut TestApp);
    let table: [(&str, Step, bool); 8] = [
        (
            "putting the tool down",
            |app| _ = app.tool(Tool::Select),
            true,
        ),
        (
            "the first Escape only puts the tool down",
            |app| _ = app.key(Key::Escape),
            true,
        ),
        (
            "the second Escape",
            |app| _ = app.key(Key::Escape).key(Key::Escape),
            false,
        ),
        (
            "the page navigating",
            |app| _ = app.page_reports("p1", PageNotice::Url("https://other.test/".into())),
            false,
        ),
        (
            "the page moving to a hash of its address",
            |app| {
                _ = app.page_reports("p1", PageNotice::Url("https://example.com/p1#top".into()));
            },
            true,
        ),
        (
            "another page navigating",
            |app| _ = app.page_reports("p2", PageNotice::Url("https://other.test/".into())),
            true,
        ),
        (
            "the page deleted",
            |app| _ = app.select(&["p1"]).act(Action::Delete),
            false,
        ),
        (
            "the page deleted and brought back",
            |app| _ = app.select(&["p1"]).act(Action::Delete).undo(),
            false,
        ),
    ];
    for (name, step, kept) in table {
        let mut app = picked_and_hovering();
        step(&mut app);
        assert_eq!(app.app().inspected().is_some(), kept, "{name}");
    }
    // Only the pick outlives the tool: back in hand, it is what shows.
    let mut app = picked_and_hovering();
    app.tool(Tool::Select);
    assert_eq!(app.inspect_model(), None);
    app.tool(Tool::Inspect);
    assert_eq!(remainder(&app).as_deref(), Some("#cell-0-1.cell"));
}

#[test]
fn a_picked_node_on_a_bound_origin_writes_to_that_repo() {
    let mut app = armed();
    app.with_chat_panel();
    app.bind(
        "https://example.com",
        "/scratch/a-long/path/to/the/site-repo",
        false,
    );
    app.click(CELL_0_1).answer_inspect();
    let composer = app.chat().composer;
    assert_eq!(
        (
            composer.folder.as_deref(),
            composer.auto.as_ref().map(|chip| chip.on)
        ),
        (Some("site-repo"), Some(false)),
        "the chips name the repo and its send mode"
    );
    app.take_effects();
    app.send_chat("make it blue");
    let request = app
        .take_effects()
        .into_iter()
        .find_map(|effect| match effect {
            Effect::RunAgent(request) => Some(request),
            _ => None,
        })
        .expect("Send runs the agent");
    assert_eq!(
        request.cwd.as_deref(),
        Some("/scratch/a-long/path/to/the/site-repo")
    );
    assert!(
        request.prompt.starts_with(
            "Working directory (linked repo): /scratch/a-long/path/to/the/site-repo\n"
        ),
        "{}",
        request.prompt
    );
    app.tool(Tool::Select).select(&["p1"]).with_panels();
    let folder = (app.popup_snapshot().lines())
        .find(|line| line.contains("page.repo.folder"))
        .map(str::to_owned);
    assert!(
        folder.is_some_and(|line| line.contains("\"\u{2026}-long/path/to/the/site-repo\"")),
        "a long folder is cut from the front in the popup's row"
    );
}
