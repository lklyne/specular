//! A text field in a panel is edited by the one text editor: a click puts the
//! caret where it landed, Enter and a press elsewhere keep what was typed,
//! Escape puts the old value back, and nothing typed reaches the canvas's
//! keys. The page popup's address and size fields are the cases.

use specular_core::PageNav;
use specular_doc::{Kind, Rect};
use specular_interact::panel::builtin::Part;
use specular_interact::{ControlId, Effect, Key, PageNotice, Tool};
use specular_testkit::{CMD, TestApp, page, sticky};

const PAGE: Rect = Rect::new(300.0, 300.0, 800.0, 500.0);

fn app() -> TestApp {
    let mut app = TestApp::with_entities([
        page("p", PAGE),
        sticky("t", Rect::new(100.0, 100.0, 200.0, 200.0), "note"),
    ]);
    app.with_panels().select(&["p"]);
    app
}

fn navigations(app: &mut TestApp) -> Vec<PageNav> {
    app.take_effects()
        .into_iter()
        .filter_map(|effect| match effect {
            Effect::Navigate { nav, .. } => Some(nav),
            _ => None,
        })
        .collect()
}

fn no_navigation() -> Vec<PageNav> {
    Vec::new()
}

fn size_of(app: &TestApp, id: &str) -> (f64, f64) {
    let rect = app.rect(id);
    (rect.width, rect.height)
}

#[test]
fn enter_sends_the_page_to_the_completed_address() {
    let mut app = app();
    app.take_effects();
    app.enter_in_field("page.url", "example.org/docs");
    assert_eq!(
        navigations(&mut app),
        [PageNav::To("https://example.org/docs".to_owned())]
    );
    assert!(app.field_edit().is_none(), "Enter ends the edit");
    assert!(app.session().editing.is_none());

    app.enter_in_field("page.url", "blue shoes");
    assert_eq!(
        navigations(&mut app),
        [PageNav::To(
            "https://www.google.com/search?q=blue%20shoes".to_owned()
        )],
        "words that are not an address become a search"
    );
}

#[test]
fn escape_puts_the_old_value_back_and_navigates_nowhere() {
    let mut app = app();
    app.take_effects();
    app.click_control("page.url")
        .chord(CMD, Key::Char('a'))
        .type_text("elsewhere.test")
        .key(Key::Escape);
    assert!(app.field_edit().is_none());
    assert_eq!(navigations(&mut app), no_navigation());
    assert_eq!(
        app.selected_ids(),
        ["p"],
        "Escape in a field keeps the page"
    );
    assert_eq!(app.session().tool, Tool::Select);
}

#[test]
fn a_press_elsewhere_keeps_what_was_typed_as_a_blur_does() {
    let mut app = app();
    app.take_effects();
    app.click_control("page.url")
        .chord(CMD, Key::Char('a'))
        .type_text("example.net");
    app.click((150.0, 150.0));
    assert!(app.field_edit().is_none());
    assert_eq!(
        navigations(&mut app),
        [PageNav::To("https://example.net/".to_owned())]
    );
}

#[test]
fn a_press_on_a_button_keeps_the_text_and_then_presses_it() {
    let mut app = app();
    app.page_reports(
        "p",
        PageNotice::Loading {
            loading: false,
            can_go_back: true,
            can_go_forward: false,
        },
    );
    app.take_effects();
    app.click_control("page.url")
        .chord(CMD, Key::Char('a'))
        .type_text("example.net")
        .click_control("page.back");
    assert_eq!(
        navigations(&mut app),
        [
            PageNav::To("https://example.net/".to_owned()),
            PageNav::Back
        ]
    );
}

#[test]
fn typing_in_a_field_never_reaches_the_canvas_keys() {
    let mut app = app();
    app.click_control("page.url");
    // `t`, `p` and Delete arm a tool or delete the selection on the canvas.
    app.type_text("tp").key(Key::Backspace).key(Key::Delete);
    assert_eq!(app.session().tool, Tool::Select);
    assert_eq!(app.selected_ids(), ["p"]);
    assert!(app.document().entity(&"p".into()).is_some());
    assert!(
        app.panel_layout().popup.is_some(),
        "the popup stays up while its field is edited"
    );
}

#[test]
fn a_click_in_the_text_puts_the_caret_where_it_landed() {
    let mut app = app();
    app.click_control("page.url")
        .chord(CMD, Key::Char('a'))
        .type_text("abcdef");
    let area = app.control_rect("page.url");
    // The measure here is 10 units a character, and the text starts a
    // border and the side padding in from the box.
    let start = area.x + 1.0 + 8.0;
    let line = area.centre().y;
    app.click((start + 22.0, line));
    assert_eq!(app.caret(), (2, 2));
    assert!(
        app.field_edit().is_some(),
        "a click in the field keeps editing"
    );
    app.double_click((start + 22.0, line));
    assert_eq!(app.caret(), (6, 0), "a double click takes the word");
}

#[test]
fn a_long_line_scrolls_to_keep_the_caret_in_view_and_back_when_it_goes_home() {
    let mut app = app();
    app.click_control("page.url");
    let long = "a".repeat(120);
    app.chord(CMD, Key::Char('a')).type_text(&long);
    let layout = app.panel_layout();
    let Some(node) = layout.node(&"page.url".to_owned().into()) else {
        panic!("no address field");
    };
    let Part::Input(input) = &node.parts[0] else {
        panic!("a field draws an input");
    };
    assert!(
        input.scroll > 0.0,
        "the caret at the end is brought into view"
    );
    let Some(caret) = input.focus.as_ref().and_then(|focus| focus.caret) else {
        panic!("no caret");
    };
    assert!(
        caret.right() <= input.area.right() + 1.0 && caret.x >= input.area.x,
        "the caret stays inside the text box"
    );
    // Home brings the start back.
    app.key(Key::Home);
    let layout = app.panel_layout();
    let Some(node) = layout.node(&"page.url".to_owned().into()) else {
        panic!("no address field");
    };
    let Part::Input(input) = &node.parts[0] else {
        panic!("a field draws an input");
    };
    assert_eq!(input.scroll, 0.0);
}

#[test]
fn back_and_forward_follow_the_pages_history_and_reload_becomes_stop_while_loading() {
    let mut app = app();
    let enabled = |app: &TestApp, id: &str| {
        let layout = app.panel_layout();
        layout
            .node(&id.to_owned().into())
            .is_some_and(|node| node.state.enabled)
    };
    assert!(!enabled(&app, "page.back"));
    assert!(!enabled(&app, "page.forward"));
    app.page_reports(
        "p",
        PageNotice::Loading {
            loading: true,
            can_go_back: true,
            can_go_forward: false,
        },
    );
    assert!(enabled(&app, "page.back"));
    assert!(!enabled(&app, "page.forward"));
    app.take_effects();
    app.click_control("page.reload");
    assert_eq!(navigations(&mut app), [PageNav::Stop], "stop while loading");
    app.page_reports(
        "p",
        PageNotice::Loading {
            loading: false,
            can_go_back: true,
            can_go_forward: true,
        },
    );
    app.click_control("page.reload").click_control("page.back");
    assert_eq!(navigations(&mut app), [PageNav::Reload, PageNav::Back]);
}

#[test]
fn the_size_fields_in_the_dropdown_set_a_custom_size_in_one_step_each() {
    let mut app = app();
    app.click_control("page.size");
    app.enter_in_field("page.size.width", "1024");
    assert_eq!(size_of(&app, "p"), (1024.0, 500.0));
    assert_eq!(
        app.session().panel.open.as_ref().map(ControlId::as_str),
        Some("page.size"),
        "the list stays open to type the other side"
    );
    app.enter_in_field("page.size.height", "768");
    assert_eq!(size_of(&app, "p"), (1024.0, 768.0));
    let Kind::Page(page) = &app.entity("p").kind else {
        panic!("a page");
    };
    let Some(meta) = &page.metadata else {
        panic!("a custom size is recorded");
    };
    assert_eq!(meta["pageSizeMode"], "custom");
    app.undo();
    assert_eq!(size_of(&app, "p"), (1024.0, 500.0));
    app.undo();
    assert_eq!(size_of(&app, "p"), (800.0, 500.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_size_that_is_not_a_number_restores_the_old_one() {
    let mut app = app();
    app.click_control("page.size");
    for text in ["wide", "0", "-5", ""] {
        app.enter_in_field("page.size.width", text);
        assert_eq!(size_of(&app, "p"), (800.0, 500.0), "{text:?}");
        assert!(app.field_edit().is_none());
    }
    assert!(!app.app().can_undo());
}

#[test]
fn the_page_tool_makes_pages_at_the_preset_it_was_given() {
    let mut app = TestApp::empty();
    app.with_panels().click_control("tool.page");
    app.click_control("page.preset.6");
    assert_eq!(app.session().tool, Tool::AddPage, "choosing keeps the tool");
    app.click((400.0, 400.0));
    let id = app.selected_ids()[0].to_owned();
    assert_eq!(size_of(&app, &id), (1280.0, 800.0));
    let Kind::Page(page) = &app.entity(&id).kind else {
        panic!("a page");
    };
    assert_eq!(page.preset_index, Some(6));
    let Some(meta) = &page.metadata else {
        panic!("metadata");
    };
    assert_eq!(meta["deviceId"], "laptop");
    assert_eq!(meta["deviceOrientation"], "landscape");
    app.assert_undo_returns_to_start();
}
